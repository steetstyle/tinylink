//! OTLP log record model, verified against a real export.
//!
//! # How this was derived
//!
//! The shape below was read off an actual `AddOtlpExporter` payload from the
//! OpenTelemetry .NET SDK 1.18.0, captured by running a real application and
//! decoding the bytes. Three findings changed this model, and each one contradicts
//! what the schema documentation alone would suggest:
//!
//! - **The trace id arrives twice, and the field outranks the scope attribute.**
//!   Fields 9 and 10 carry `trace_id` and `span_id` as raw bytes, and the SDK also
//!   copies them into the log scope as `TraceId`/`SpanId` attributes — and the
//!   scope copies come *last* in the wire order, so a parser that merges both into
//!   one attribute map silently prefers whichever it saw last. An application's
//!   logging scope is user input; the field comes from the activity. See
//!   [`LogRecord::trace_id`].
//! - **Timestamps are fixed64, not varints**, in fields 1 and 11. Field 8 is a
//!   fixed32 carrying trace *flags*, and reading it as a time gives a plausible
//!   number that is wrong by six orders of magnitude.
//! - **`severity_number` and `severity_text` are both present** and do not always
//!   agree, so [`Severity::from_number`] falls back to the text when the number is
//!   unusable.
//!
//! The SDK also emits the default OpenTelemetry resource attributes
//! (`service.name`, `telemetry.sdk.*`), which is where the service name comes
//! from — traces and logs must agree on it or a join silently fails.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One decoded log record, flattened to what a query actually needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogRecord {
    /// Log body, already rendered to a string.
    pub body: String,
    /// Numeric severity, 1=Trace … 24=Fatal, 0 when unknown.
    pub severity_number: u32,
    /// Severity as text, e.g. `Information`.
    pub severity_text: String,
    /// Attributes, including any carried by an enclosing log scope.
    pub attributes: std::collections::BTreeMap<String, String>,
    /// Unix nanoseconds.
    pub time_unix_nano: u64,
    pub observed_time_unix_nano: u64,
    /// The dedicated OTLP trace id field, when the exporter sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// The dedicated OTLP span id field, when the exporter sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

/// First non-empty value among `keys`.
fn lookup<'a>(
    attributes: &'a std::collections::BTreeMap<String, String>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .filter_map(|k| attributes.get(*k))
        .find(|v| !v.is_empty())
        .map(|v| v.as_str())
}

impl LogRecord {
    /// The trace this record belongs to, if it carries one.
    ///
    /// The dedicated OTLP field wins over a scope attribute of the same name.
    /// Both are usually present on the .NET SDK: the field is populated from the
    /// activity, while the attribute comes from whatever the application put in
    /// its logging scope. An application scope is user input, so when the two
    /// disagree the activity is the more trustworthy source.
    ///
    /// Several spellings are accepted on the attribute side, because a service
    /// instrumented differently still has to join.
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| lookup(&self.attributes, &["TraceId", "traceId", "trace_id"]))
    }

    /// The span this record belongs to. See [`LogRecord::trace_id`] for why the
    /// dedicated field wins.
    pub fn span_id(&self) -> Option<&str> {
        self.span_id
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| lookup(&self.attributes, &["SpanId", "spanId", "span_id"]))
    }

    pub fn severity(&self) -> Severity {
        Severity::from_number(self.severity_number, &self.severity_text)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "body": self.body,
            "severity": self.severity().as_str(),
            "severity_number": self.severity_number,
            "trace_id": self.trace_id(),
            "span_id": self.span_id(),
            "attributes": self.attributes,
            "time_unix_nano": self.time_unix_nano.to_string(),
        })
    }
}

/// Log severity, as OTLP numbers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Trace => "Trace",
            Severity::Debug => "Debug",
            Severity::Info => "Info",
            Severity::Warn => "Warn",
            Severity::Error => "Error",
            Severity::Fatal => "Fatal",
            Severity::Unknown => "Unknown",
        }
    }

    /// OTLP severity numbers.
    pub fn number(&self) -> u32 {
        match self {
            Severity::Trace => 1,
            Severity::Debug => 5,
            Severity::Info => 9,
            Severity::Warn => 13,
            Severity::Error => 17,
            Severity::Fatal => 21,
            Severity::Unknown => 0,
        }
    }

    /// Interpret a record, preferring the text when the number is unusable.
    ///
    /// The two disagree often enough to be worth handling: a record with a text
    /// like `Error` and a missing number should still be treated as an error, and
    /// some exporters send only one of the two.
    pub fn from_number(number: u32, text: &str) -> Self {
        if number > 0 {
            return match number {
                1..=4 => Severity::Trace,
                5..=8 => Severity::Debug,
                9..=12 => Severity::Info,
                13..=16 => Severity::Warn,
                17..=20 => Severity::Error,
                21..=24 => Severity::Fatal,
                _ => Severity::Unknown,
            };
        }
        match text.trim().to_lowercase().as_str() {
            "trace" | "verbose" => Severity::Trace,
            "debug" => Severity::Debug,
            "info" | "information" => Severity::Info,
            "warn" | "warning" => Severity::Warn,
            "error" | "err" => Severity::Error,
            "fatal" | "critical" | "crit" => Severity::Fatal,
            _ => Severity::Unknown,
        }
    }

    /// Is this severity at or above `min`?
    ///
    /// `Unknown` never counts as a match: an unparseable record must not be
    /// reported as an error just because it sorts high.
    pub fn at_least(&self, min: Severity) -> bool {
        // Unknown never satisfies a threshold: an unparseable record must not
        // be reported as an error just because it sorts high.
        if *self == Severity::Unknown || min == Severity::Unknown {
            return false;
        }
        self.number() >= min.number()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(severity_number: u32, severity_text: &str, attrs: &[(&str, &str)]) -> LogRecord {
        LogRecord {
            body: "test".into(),
            severity_number,
            severity_text: severity_text.into(),
            attributes: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            time_unix_nano: 0,
            observed_time_unix_nano: 0,
            trace_id: None,
            span_id: None,
        }
    }

    /// The dedicated OTLP field outranks a scope attribute of the same name.
    ///
    /// Both are present on a real .NET export and they agree there, so this only
    /// shows up when an application puts a different value in its logging scope.
    /// The activity-derived field is the one the collector's trace index uses, so
    /// matching it is what keeps a log joinable.
    #[test]
    fn dedicated_trace_id_field_wins_over_the_scope_attribute() {
        let mut r = record(9, "Info", &[("TraceId", "from-scope")]);
        r.trace_id = Some("from-field".into());
        assert_eq!(r.trace_id(), Some("from-field"));
    }

    /// An empty dedicated field is not a trace: falling back would otherwise let
    /// `""` shadow a perfectly good scope value.
    #[test]
    fn an_empty_trace_id_field_falls_back_to_the_attribute() {
        let mut r = record(9, "Info", &[("TraceId", "from-scope")]);
        r.trace_id = Some(String::new());
        assert_eq!(r.trace_id(), Some("from-scope"));
    }

    #[test]
    fn severity_numbers_map_to_the_expected_bands() {
        assert_eq!(Severity::from_number(1, ""), Severity::Trace);
        assert_eq!(Severity::from_number(9, ""), Severity::Info);
        assert_eq!(Severity::from_number(17, ""), Severity::Error);
        assert_eq!(Severity::from_number(21, ""), Severity::Fatal);
    }

    /// A record whose number is missing must still be classified by its text.
    /// Otherwise a whole exporter's errors vanish from a severity filter.
    #[test]
    fn severity_text_is_used_when_the_number_is_missing() {
        assert_eq!(Severity::from_number(0, "Error"), Severity::Error);
        assert_eq!(Severity::from_number(0, "warning"), Severity::Warn);
        assert_eq!(Severity::from_number(0, "Information"), Severity::Info);
        assert_eq!(Severity::from_number(0, "nonsense"), Severity::Unknown);
    }

    #[test]
    fn at_least_compares_severity() {
        assert!(Severity::Error.at_least(Severity::Info));
        assert!(Severity::Error.at_least(Severity::Error));
        assert!(!Severity::Info.at_least(Severity::Error));
        assert!(Severity::Warn.at_least(Severity::Info));
    }

    /// An unparseable record must never be promoted to a match. Otherwise a
    /// severity filter reports records it cannot actually classify.
    #[test]
    fn unknown_severity_never_satisfies_a_threshold() {
        assert!(!Severity::Unknown.at_least(Severity::Trace));
        assert!(!Severity::Unknown.at_least(Severity::Error));
    }

    /// The join to a trace goes through the scope attribute, because the .NET SDK
    /// emits no trace_id field.
    #[test]
    fn trace_id_comes_from_the_scope_attribute() {
        let r = record(
            9,
            "Information",
            &[("TraceId", "abc123"), ("SpanId", "def456")],
        );
        assert_eq!(r.trace_id(), Some("abc123"));
        assert_eq!(r.span_id(), Some("def456"));
    }

    #[test]
    fn trace_id_accepts_alternative_spellings() {
        assert_eq!(record(9, "Info", &[("traceId", "x")]).trace_id(), Some("x"));
        assert_eq!(
            record(9, "Info", &[("trace_id", "y")]).trace_id(),
            Some("y")
        );
    }

    /// A record logged outside any request scope must report no trace rather than
    /// an empty string that looks like a join key.
    #[test]
    fn an_unscoped_record_has_no_trace() {
        let r = record(9, "Debug", &[("Other", "value")]);
        assert_eq!(r.trace_id(), None);
        let empty = record(9, "Debug", &[("TraceId", "")]);
        assert_eq!(empty.trace_id(), None, "an empty id is not a trace");
    }

    #[test]
    fn to_json_exposes_what_a_query_needs() {
        let r = record(17, "Error", &[("TraceId", "t1"), ("Exception", "boom")]);
        let v = r.to_json();
        assert_eq!(v["severity"], "Error");
        assert_eq!(v["trace_id"], "t1");
        assert_eq!(v["attributes"]["Exception"], "boom");
        // Nanosecond timestamps exceed what JSON numbers represent safely.
        assert!(v["time_unix_nano"].is_string());
    }
}
