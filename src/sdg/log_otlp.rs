//! Parsing OTLP log exports.
//!
//! Both wire formats are accepted:
//!
//! - **JSON**, as the OTLP/HTTP spec's `/v1/logs` endpoint defines it.
//! - **protobuf**, which is what the OpenTelemetry .NET SDK sends by default.
//!
//! The protobuf reader is hand-rolled rather than generated. It only decodes the
//! fields a query needs, and the field numbers were read off a real export — see
//! [`super::log`] for what that turned up, notably that log records carry no
//! `trace_id` field at all.
//!
//! Only uncompressed and gzip payloads are handled, which is what the SDK sends.

use std::collections::BTreeMap;
use std::io::Read;

use anyhow::{Context, Result};

use super::log::LogRecord;

/// Logs from one exporter, with the resource attributes they arrived with.
///
/// The resource attributes matter: `service.name` is the only reliable link
/// between a log and the service names in a trace, and a join that guesses it
/// silently attributes errors to the wrong service.
#[derive(Debug, Clone, Default)]
pub struct LogBatch {
    pub records: Vec<LogRecord>,
    pub resource_attributes: BTreeMap<String, String>,
}

impl LogBatch {
    pub fn service_name(&self) -> Option<&str> {
        self.resource_attributes
            .iter()
            .find(|(k, _)| k.as_str() == "service.name" || k.as_str() == "serviceName")
            .map(|(_, v)| v.as_str())
    }
}

// ───────────────────────── JSON ─────────────────────────

/// Parse an OTLP/JSON logs payload.
pub fn parse_json(bytes: &[u8]) -> Result<LogBatch> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).context("logs payload is not valid JSON")?;
    Ok(parse_json_value(&value))
}

pub fn parse_json_value(root: &serde_json::Value) -> LogBatch {
    let mut batch = LogBatch::default();

    let resource_logs = root
        .get("resourceLogs")
        .or_else(|| root.get("resource_logs"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for rl in resource_logs {
        if let Some(attrs) = rl
            .get("resource")
            .and_then(|r| r.get("attributes"))
            .and_then(|a| a.as_array())
        {
            for (k, v) in attributes_to_map(attrs) {
                batch.resource_attributes.insert(k, v);
            }
        }

        let scope_logs = rl
            .get("scopeLogs")
            .or_else(|| rl.get("scope_logs"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for sl in scope_logs {
            let entries = sl
                .get("logRecords")
                .or_else(|| sl.get("log_records"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for entry in entries {
                batch.records.push(json_record(&entry));
            }
        }
    }

    batch
}

fn attributes_to_map(list: &[serde_json::Value]) -> Vec<(String, String)> {
    list.iter()
        .filter_map(|kv| {
            let key = kv.get("key")?.as_str()?.to_string();
            let value = kv.get("value")?;
            Some((key, any_value_to_string(value)))
        })
        .collect()
}

/// OTLP/JSON AnyValue: a one-key object selecting the type.
fn any_value_to_string(value: &serde_json::Value) -> String {
    if let Some(s) = value
        .get("stringValue")
        .or_else(|| value.get("string_value"))
    {
        return s.as_str().unwrap_or_default().to_string();
    }
    for key in [
        "intValue",
        "int_value",
        "doubleValue",
        "double_value",
        "boolValue",
        "bool_value",
    ] {
        if let Some(v) = value.get(key) {
            return match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
        }
    }
    if let Some(array) = value.get("arrayValue").or_else(|| value.get("array_value")) {
        if let Some(values) = array.get("values") {
            return values
                .as_array()
                .map(|v| {
                    v.iter()
                        .map(any_value_to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
        }
    }
    // Last resort: the SDK sometimes puts the scalar directly under `value`.
    value
        .get("value")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn json_record(entry: &serde_json::Value) -> LogRecord {
    let mut attributes = BTreeMap::new();
    if let Some(list) = entry.get("attributes").and_then(|v| v.as_array()) {
        for (k, v) in attributes_to_map(list) {
            attributes.insert(k, v);
        }
    }

    let body = entry
        .get("body")
        .map(any_value_to_string)
        .unwrap_or_default();

    let severity_text = entry
        .get("severityText")
        .or_else(|| entry.get("severity_text"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let severity_number = entry
        .get("severityNumber")
        .or_else(|| entry.get("severity_number"))
        .and_then(json_number)
        .map(|v| v.min(24) as u32)
        .unwrap_or(0);

    // Some exporters put the ids in dedicated fields, others only in the scope
    // attributes. Read the fields when present and leave the rest to
    // LogRecord::trace_id, which falls back to the attributes.
    let field_value = |keys: [&str; 2]| {
        keys.iter()
            .find_map(|k| entry.get(*k).and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    };
    let trace_id = field_value(["traceId", "trace_id"]);
    let span_id = field_value(["spanId", "span_id"]);

    let time = entry
        .get("timeUnixNano")
        .or_else(|| entry.get("time_unix_nano"))
        .and_then(json_number)
        .unwrap_or(0);
    let observed = entry
        .get("observedTimeUnixNano")
        .or_else(|| entry.get("observed_time_unix_nano"))
        .and_then(json_number)
        .unwrap_or(time);

    LogRecord {
        body,
        severity_number,
        severity_text,
        attributes,
        time_unix_nano: time,
        observed_time_unix_nano: observed,
        trace_id,
        span_id,
    }
}

/// OTLP/JSON encodes 64-bit integers as strings to survive JavaScript.
fn json_number(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

// ───────────────────────── protobuf ─────────────────────────

/// Parse an OTLP/protobuf logs payload.
pub fn parse_protobuf(bytes: &[u8]) -> Result<LogBatch> {
    let mut batch = LogBatch::default();
    for (field, wire, value) in Fields::new(bytes) {
        if field == 1 && wire == 2 {
            if let Some(bytes) = value.into_bytes() {
                resource_logs_into(&bytes, &mut batch);
            }
        }
    }
    Ok(batch)
}

fn resource_logs_into(buf: &[u8], batch: &mut LogBatch) {
    // ResourceLogs: 1 Resource, 2 ScopeLogs, 3 schema_url.
    for (field, wire, value) in Fields::new(buf) {
        match (field, wire) {
            (1, 2) => {
                if let Some(resource) = value.as_bytes() {
                    for (f, w, v) in Fields::new(resource) {
                        if f == 1 && w == 2 {
                            if let Some(kv) = v.as_bytes() {
                                if let Some((k, val)) = key_value(kv) {
                                    batch.resource_attributes.insert(k, val);
                                }
                            }
                        }
                    }
                }
            }
            (2, 2) => {
                if let Some(scope_logs) = value.as_bytes() {
                    for (f, w, v) in Fields::new(scope_logs) {
                        // Inside ScopeLogs: 1 InstrumentationScope, 2 LogRecord
                        // (repeated), 3 schema_url. Reading 3 here looks right and
                        // silently yields zero records.
                        if f == 2 && w == 2 {
                            if let Some(record) = v.as_bytes() {
                                batch.records.push(protobuf_record(record));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Decode one LogRecord.
///
/// Field numbers read off a real export from the OpenTelemetry .NET SDK, not
/// from documentation. Three of them are easy to get wrong, and were: 9 is
/// trace_id rather than a second body, 8 is flags rather than the timestamp, and
/// 11 rather than 1 is observed time.
fn protobuf_record(buf: &[u8]) -> LogRecord {
    let mut attributes: BTreeMap<String, String> = BTreeMap::new();
    let mut body = String::new();
    let mut severity_text = String::new();
    let mut severity_number = 0u32;
    let mut time_unix_nano = 0u64;
    let mut observed = 0u64;
    let mut trace_id = None;
    let mut span_id = None;

    for (field, wire, value) in Fields::new(buf) {
        match (field, wire) {
            // time_unix_nano: fixed64
            (1, 1) => {
                if let Some(v) = value.as_int() {
                    time_unix_nano = v;
                }
            }
            // severity_number: varint
            (2, 0) => {
                if let Some(v) = value.as_int() {
                    severity_number = v.min(24) as u32;
                }
            }
            // severity_text
            (3, 2) => {
                if let Some(v) = value.as_bytes() {
                    severity_text = String::from_utf8_lossy(v).to_string();
                }
            }
            // body (AnyValue)
            (5, 2) => {
                if let Some(v) = value.as_bytes() {
                    body = any_value_protobuf(v);
                }
            }
            // attributes (repeated KeyValue)
            (6, 2) => {
                if let Some(v) = value.as_bytes() {
                    if let Some((k, val)) = key_value(v) {
                        attributes.insert(k, val);
                    }
                }
            }
            // 8 is flags (fixed32) and is deliberately ignored: it carries trace
            // flags, not a time, and reading it as one truncates the timestamp.
            // trace_id: 16 raw bytes
            (9, 2) => {
                if let Some(v) = value.as_bytes() {
                    trace_id = Some(hex(v));
                }
            }
            // span_id: 8 raw bytes
            (10, 2) => {
                if let Some(v) = value.as_bytes() {
                    span_id = Some(hex(v));
                }
            }
            // observed_time_unix_nano: fixed64
            (11, 1) => {
                if let Some(v) = value.as_int() {
                    observed = v;
                }
            }
            _ => {}
        }
    }

    LogRecord {
        body,
        severity_number,
        severity_text,
        attributes,
        time_unix_nano,
        observed_time_unix_nano: observed,
        trace_id,
        span_id,
    }
}

/// Lowercase hex, the form a trace or span id is normally written in.
///
/// Trace ids arrive as raw bytes while a trace query uses the textual form, so
/// converting here is what makes the two halves of a trace joinable at all.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// KeyValue: 1 key (string), 2 value (AnyValue).
fn key_value(buf: &[u8]) -> Option<(String, String)> {
    let mut key = None;
    let mut value = None;
    for (field, wire, payload) in Fields::new(buf) {
        match (field, wire) {
            (1, 2) => {
                if let Some(v) = payload.as_bytes() {
                    key = Some(String::from_utf8_lossy(v).to_string());
                }
            }
            (2, 2) => {
                if let Some(v) = payload.as_bytes() {
                    value = Some(any_value_protobuf(v));
                }
            }
            _ => {}
        }
    }
    Some((key?, value.unwrap_or_default()))
}

/// AnyValue: 1 stringValue, 2 boolValue, 3 intValue, 4 doubleValue.
fn any_value_protobuf(buf: &[u8]) -> String {
    for (field, wire, value) in Fields::new(buf) {
        match (field, wire) {
            (1, 2) => {
                if let Some(v) = value.as_bytes() {
                    return String::from_utf8_lossy(v).to_string();
                }
            }
            (2, 0) => return value.as_int().map(|v| v != 0).unwrap_or(false).to_string(),
            (3, 0) => return value.as_int().map(|v| v.to_string()).unwrap_or_default(),
            (4, 1) => {
                return value
                    .as_int()
                    .map(|v| format!("{}", f64::from_bits(v)))
                    .unwrap_or_default()
            }
            _ => {}
        }
    }
    String::new()
}

/// A protobuf field reader yielding owned payloads.
///
/// Varints and fixed-width fields come back as `u64`; length-delimited ones as
/// bytes. Collapsing all three into a byte buffer would mean re-encoding varints
/// just to read them, and every reader below only ever wants the raw slice or the
/// integer.
struct Fields<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Fields<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn varint(&mut self) -> Option<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            result |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
        None
    }

    fn take(&mut self, len: usize) -> Option<Vec<u8>> {
        if self.pos + len > self.buf.len() {
            return None;
        }
        let slice = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(slice)
    }
}

/// One field: its number, its wire type, and its decoded payload.
type Field = (u64, u8, Value);

/// The decoded payload of a field.
#[derive(Debug, Clone)]
enum Value {
    Varint(u64),
    Bytes(Vec<u8>),
}

impl Value {
    fn as_int(&self) -> Option<u64> {
        match self {
            Value::Varint(v) => Some(*v),
            Value::Bytes(_) => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            Value::Varint(_) => None,
        }
    }

    fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Value::Bytes(b) => Some(b),
            Value::Varint(_) => None,
        }
    }
}

/// Iterate a protobuf message, stopping at the first malformed field rather
/// than reading past the end of the buffer.
impl Iterator for Fields<'_> {
    type Item = Field;

    fn next(&mut self) -> Option<Field> {
        let key = self.varint()?;
        let field = key >> 3;
        let wire = (key & 7) as u8;
        if field == 0 {
            return None;
        }
        let value = match wire {
            0 => self.varint().map(Value::Varint)?,
            1 => {
                let raw = self.take(8)?;
                Value::Varint(u64::from_le_bytes(raw.try_into().ok()?))
            }
            2 => {
                let len = self.varint()? as usize;
                Value::Bytes(self.take(len)?)
            }
            5 => {
                let raw = self.take(4)?;
                let mut padded = [0u8; 8];
                padded[..4].copy_from_slice(&raw);
                Value::Varint(u32::from_le_bytes(padded[..4].try_into().ok()?) as u64)
            }
            // Groups are deprecated and do not appear in OTLP.
            _ => return None,
        };
        Some((field, wire, value))
    }
}

/// Parse an OTLP/HTTP response body, honouring gzip.
///
/// Content sniffing rather than a required header: a collector or proxy that
/// re-sends the payload without the header would otherwise fail silently.
pub fn parse_otlp_http(body: &[u8], content_encoding: Option<&str>) -> Result<LogBatch> {
    let gzipped = body.len() > 2 && body[0] == 0x1F && body[1] == 0x8B
        || content_encoding
            .map(|e| e.eq_ignore_ascii_case("gzip"))
            .unwrap_or(false);

    let decoded: Vec<u8> = if gzipped {
        let mut decoder = flate2::read::GzDecoder::new(body);
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .context("gzip payload could not be decompressed")?;
        out
    } else {
        body.to_vec()
    };

    // JSON starts with `{` or `[`; anything else is treated as protobuf, which
    // is what the .NET SDK sends.
    if decoded
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{')
    {
        parse_json(&decoded)
    } else {
        parse_protobuf(&decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdg::log::Severity;

    #[test]
    fn json_any_value_reads_every_scalar_form() {
        let cases = [
            (serde_json::json!({"stringValue": "hi"}), "hi"),
            (serde_json::json!({"intValue": "42"}), "42"),
            (serde_json::json!({"boolValue": true}), "true"),
            (serde_json::json!({"doubleValue": 1.5}), "1.5"),
        ];
        for (value, expected) in cases {
            assert_eq!(any_value_to_string(&value), expected);
        }
    }

    #[test]
    fn json_export_yields_records_and_resource_attributes() {
        let payload = serde_json::json!({
            "resourceLogs": [{
                "resource": {"attributes": [
                    {"key": "service.name", "value": {"stringValue": "Agency.API"}}
                ]},
                "scopeLogs": [{"logRecords": [{
                    "severityNumber": 17,
                    "severityText": "Error",
                    "body": {"stringValue": "handler failed"},
                    "timeUnixNano": "1700000000000000000",
                    "attributes": [
                        {"key": "TraceId", "value": {"stringValue": "t-1"}}
                    ]
                }]}]
            }]
        });

        let batch = parse_json_value(&payload);
        assert_eq!(batch.records.len(), 1, "{batch:?}");
        assert_eq!(batch.service_name(), Some("Agency.API"));
        let r = &batch.records[0];
        assert_eq!(r.body, "handler failed");
        assert_eq!(r.severity(), Severity::Error);
        assert_eq!(r.trace_id(), Some("t-1"));
        // Nanosecond timestamps arrive as strings in JSON.
        assert_eq!(r.time_unix_nano, 1_700_000_000_000_000_000);
    }

    /// The trace↔log join depends on the scope attribute, since the SDK emits no
    /// trace_id field. If this breaks, every log looks orphaned.
    #[test]
    fn json_trace_id_field_is_accepted_when_present() {
        let payload = serde_json::json!({
            "resourceLogs": [{"scopeLogs": [{"logRecords": [{
                "body": {"stringValue": "x"},
                "traceId": "from-field",
                "attributes": []
            }]}]}]
        });
        let batch = parse_json_value(&payload);
        assert_eq!(batch.records[0].trace_id(), Some("from-field"));
    }

    #[test]
    fn multiple_resource_logs_are_all_collected() {
        let payload = serde_json::json!({
            "resourceLogs": [
                {"resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "A"}}]},
                 "scopeLogs": [{"logRecords": [{"body": {"stringValue": "a"}}]}]},
                {"resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "B"}}]},
                 "scopeLogs": [{"logRecords": [{"body": {"stringValue": "b"}}]}]}
            ]
        });
        let batch = parse_json_value(&payload);
        assert_eq!(batch.records.len(), 2, "{batch:?}");
    }

    #[test]
    fn empty_payload_yields_an_empty_batch() {
        let batch = parse_json_value(&serde_json::json!({}));
        assert!(batch.records.is_empty());
        assert_eq!(batch.service_name(), None);
    }

    #[test]
    fn invalid_json_is_an_error_not_a_silent_empty_batch() {
        let err = parse_json(b"not json at all");
        assert!(err.is_err(), "malformed input must be reported");
    }

    // ── protobuf ──

    /// Build a protobuf message from (field, wire, payload) triples.
    fn encode(fields: &[(u64, u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (field, wire, payload) in fields {
            let key = (field << 3) | (*wire as u64);
            let mut varint = key;
            while varint >= 0x80 {
                out.push((varint as u8) | 0x80);
                varint >>= 7;
            }
            out.push(varint as u8);
            match wire {
                0 => {
                    let mut v = *payload.first().expect("varint needs a byte");
                    while v >= 0x80 {
                        out.push(v | 0x80);
                        v >>= 7;
                    }
                    out.push(v);
                }
                2 => {
                    let mut len = payload.len() as u64;
                    while len >= 0x80 {
                        out.push((len as u8) | 0x80);
                        len >>= 7;
                    }
                    out.push(len as u8);
                    out.extend_from_slice(payload);
                }
                5 => out.extend_from_slice(&payload[..4]),
                1 => out.extend_from_slice(&payload[..8]),
                _ => {}
            }
        }
        out
    }

    /// A varint field's payload is just the low byte here: `encode` performs the
    /// varint encoding itself, so pre-encoding would double it up.
    fn varint_bytes(v: u64) -> Vec<u8> {
        vec![(v & 0x7F) as u8]
    }

    fn string_field(field: u64, text: &str) -> (u64, u8, Vec<u8>) {
        (field, 2, text.as_bytes().to_vec())
    }

    /// A LogRecord shaped like the real export: severity as text, body as
    /// AnyValue, attributes carrying the scope, and time as fixed32.
    #[test]
    fn protobuf_record_decodes_the_real_export_shape() {
        let any_string = |s: &str| encode(&[string_field(1, s)]);
        let kv = |k: &str, v: &str| encode(&[string_field(1, k), (2, 2, any_string(v))]);

        let log_record = encode(&[
            string_field(3, "Error"),
            (5, 2, any_string("handler failed")),
            (6, 2, kv("TraceId", "abc123")),
            (6, 2, kv("SpanId", "def456")),
            (2, 0, varint_bytes(17)),
        ]);

        let scope_logs = encode(&[(2, 2, log_record)]);
        let resource_logs = encode(&[
            (1, 2, encode(&[(1, 2, kv("service.name", "Agency.API"))])),
            (2, 2, scope_logs),
        ]);
        let payload = encode(&[(1, 2, resource_logs)]);

        let batch = parse_protobuf(&payload).expect("parse");
        assert_eq!(batch.service_name(), Some("Agency.API"), "{batch:?}");
        assert_eq!(batch.records.len(), 1, "{batch:?}");
        let r = &batch.records[0];
        assert_eq!(r.body, "handler failed");
        assert_eq!(r.severity_text, "Error");
        assert_eq!(r.severity(), Severity::Error);
        // The join has to come from the scope attribute.
        assert_eq!(r.trace_id(), Some("abc123"), "{r:?}");
        assert_eq!(r.span_id(), Some("def456"), "{r:?}");
    }

    /// Timestamps come from field 1 (fixed64) and field 11 (fixed64).
    ///
    /// Field 8 is a fixed32 carrying trace *flags*, and reading it as the
    /// timestamp yields a plausible-looking number that is wrong by six orders of
    /// magnitude — the kind of bug that survives a look at the output.
    #[test]
    fn protobuf_timestamps_come_from_the_fixed64_fields() {
        let any_string = |s: &str| encode(&[string_field(1, s)]);
        let time = 1_700_000_000_123_456_789u64;
        let observed = 1_700_000_000_123_999_999u64;
        let log_record = encode(&[
            (2, 0, varint_bytes(9)),
            (5, 2, any_string("hello")),
            (1, 1, time.to_le_bytes().to_vec()),
            (11, 1, observed.to_le_bytes().to_vec()),
            // flags: must not be mistaken for a timestamp
            (8, 5, vec![0x01, 0x00, 0x00, 0x00]),
        ]);
        let scope_logs = encode(&[(2, 2, log_record)]);
        let payload = encode(&[(1, 2, encode(&[(2, 2, scope_logs)]))]);

        let batch = parse_protobuf(&payload).expect("parse");
        assert_eq!(batch.records[0].time_unix_nano, time);
        assert_eq!(batch.records[0].observed_time_unix_nano, observed);
    }

    /// A record with no timestamp at all reports zero rather than inventing one.
    /// Zero is visibly wrong (1970), which is the point: a caller can see the gap
    /// instead of trusting a fabricated ordering.
    #[test]
    fn protobuf_record_without_a_timestamp_reports_zero() {
        let any_string = |s: &str| encode(&[string_field(1, s)]);
        let log_record = encode(&[(5, 2, any_string("no clock"))]);
        let scope_logs = encode(&[(2, 2, log_record)]);
        let payload = encode(&[(1, 2, encode(&[(2, 2, scope_logs)]))]);

        let batch = parse_protobuf(&payload).expect("parse");
        assert_eq!(batch.records[0].time_unix_nano, 0);
    }

    /// trace_id and span_id arrive as raw bytes in fields 9 and 10.
    ///
    /// The exporter also copies them into the log scope, so both routes are
    /// usually present; the dedicated field wins, because it is the one the SDK
    /// populates from the activity rather than from user-supplied scope values.
    #[test]
    fn protobuf_trace_and_span_ids_are_read_as_hex() {
        let any_string = |s: &str| encode(&[string_field(1, s)]);
        let kv = |k: &str, v: &str| encode(&[string_field(1, k), (2, 2, any_string(v))]);

        let trace = [
            0xcd, 0xbe, 0xa3, 0x4d, 0xb5, 0xc9, 0x1d, 0xcd, 0x92, 0xfc, 0x40, 0x93, 0xeb, 0xf2,
            0x45, 0x6b,
        ];
        let span = [0xe3, 0x39, 0x95, 0xf5, 0xe5, 0xd9, 0xca, 0x1d];

        let log_record = encode(&[
            (5, 2, any_string("boom")),
            (9, 2, trace.to_vec()),
            (10, 2, span.to_vec()),
            // A stale scope value that must not win.
            (6, 2, kv("TraceId", "from-scope")),
        ]);
        let scope_logs = encode(&[(2, 2, log_record)]);
        let payload = encode(&[(1, 2, encode(&[(2, 2, scope_logs)]))]);

        let batch = parse_protobuf(&payload).expect("parse");
        let r = &batch.records[0];
        assert_eq!(
            r.trace_id(),
            Some("cdbea34db5c91dcd92fc4093ebf2456b"),
            "{r:?}"
        );
        assert_eq!(r.span_id(), Some("e33995f5e5d9ca1d"), "{r:?}");
    }

    /// Truncated input must not panic: a decoder that trusts the buffer will
    /// read past the end and take the process with it.
    #[test]
    fn truncated_protobuf_does_not_panic() {
        let log_record = encode(&[(2, 0, varint_bytes(9))]);
        let scope_logs = encode(&[(2, 2, log_record)]);
        let full = encode(&[(1, 2, encode(&[(2, 2, scope_logs)]))]);
        for cut in 1..full.len() {
            let _ = parse_protobuf(&full[..cut]);
        }
    }

    #[test]
    fn garbage_protobuf_yields_an_empty_batch_not_a_panic() {
        let batch = parse_protobuf(&[0xFF, 0xFF, 0xFF, 0xFF]).expect("parse");
        assert!(batch.records.is_empty());
    }

    #[test]
    fn gzip_payloads_are_decompressed() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let payload = serde_json::json!({
            "resourceLogs": [{"scopeLogs": [{"logRecords": [{
                "body": {"stringValue": "gzipped"}, "severityText": "Warning"
            }]}]}]
        });
        let raw = serde_json::to_vec(&payload).unwrap();

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();

        let batch = parse_otlp_http(&compressed, Some("gzip")).expect("parse gzipped");
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.records[0].body, "gzipped");
        assert_eq!(batch.records[0].severity(), Severity::Warn);
    }

    #[test]
    fn uncompressed_json_is_detected_without_a_header() {
        let payload = b"{\"resourceLogs\":[{\"scopeLogs\":[{\"logRecords\":[{\"body\":{\"stringValue\":\"x\"}}]}]}]}";
        let batch = parse_otlp_http(payload, None).expect("parse");
        assert_eq!(batch.records.len(), 1);
    }
}
