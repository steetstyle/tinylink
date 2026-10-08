//! The parser, run against bytes an actual OpenTelemetry .NET SDK produced.
//!
//! `fixtures/otlp-logs-capture.bin` is the raw request body of two OTLP/protobuf
//! exports from a running .NET application. Every other test in this crate builds
//! its own protobuf, which means it can only prove the parser agrees with the
//! test author. This one proves the parser agrees with the SDK.

use tinylink::sdg::log::Severity;
use tinylink::sdg::log_otlp::parse_otlp_http;

fn capture() -> tinylink::sdg::log_otlp::LogBatch {
    let bytes = include_bytes!("fixtures/otlp-logs-capture.bin");
    parse_otlp_http(bytes, None).expect("capture must parse")
}

#[test]
fn the_service_name_comes_from_the_resource_attributes() {
    let batch = capture();
    // This is the join key against traces. If it is missing, every log is
    // unattributable to a service and the whole feature is inert.
    assert_eq!(batch.service_name(), Some("Agency.API"));
    assert_eq!(
        batch.resource_attributes.get("telemetry.sdk.language"),
        Some(&"dotnet".to_string()),
        "the SDK's own resource attributes should survive: {:?}",
        batch.resource_attributes
    );
}

#[test]
fn every_record_in_the_capture_is_decoded() {
    let batch = capture();
    // Four records per export, two exports. A field-number mistake in the
    // resource/scope walk shows up here as a much smaller count rather than as a
    // panic, which is why the count is asserted rather than just non-emptiness.
    assert_eq!(batch.records.len(), 8, "{:?}", batch.records);
}

/// The SDK exports the *message template* as the body, not the rendered text.
///
/// This is the single most consequential fact about the schema, because it means
/// a substring search over the body alone misses every interpolated value. A log
/// search that only looks at `body` would fail to find "no name for agency 9f1c"
/// even though the log containing it is right there.
#[test]
fn the_body_is_the_unrendered_message_template() {
    let batch = capture();
    let error = batch
        .records
        .iter()
        .find(|r| r.severity() == Severity::Error)
        .expect("an error record");

    assert_eq!(error.body, "Handler {HandlerName} failed: {Message}");
    // The values live in attributes instead, which is where a search has to look.
    assert_eq!(
        error.attributes.get("HandlerName"),
        Some(&"GetAgencyMembers".to_string())
    );
    assert_eq!(
        error.attributes.get("Message"),
        Some(&"no name for agency 9f1c".to_string())
    );
}

/// The exception survives the round trip, type and stack included.
///
/// This is what turns a log query into a debugging entry point: the type says
/// what was thrown and the stack says where.
#[test]
fn exception_type_message_and_stack_are_preserved() {
    let batch = capture();
    let error = batch
        .records
        .iter()
        .find(|r| r.severity() == Severity::Error)
        .expect("an error record");

    assert_eq!(
        error.attributes.get("exception.type"),
        Some(&"InvalidOperationException".to_string()),
        "{error:?}"
    );
    assert_eq!(
        error.attributes.get("exception.message"),
        Some(&"no name for agency 9f1c".to_string()),
        "{error:?}"
    );
    let stack = error
        .attributes
        .get("exception.stacktrace")
        .expect("a stack trace is what makes the log actionable");
    assert!(stack.contains("InvalidOperationException"), "{stack}");
    assert!(stack.contains("Program.cs"), "{stack}");
}

/// Every record carries a trace id even the one logged outside a logging scope.
///
/// The scope copies the id into a `TraceId` attribute; the dedicated field is
/// filled from the ambient activity. A record logged with no `BeginScope` still
/// has the field, which is why the field is the primary source and the attribute
/// only a fallback.
#[test]
fn all_records_are_traceable_including_the_unscoped_one() {
    let batch = capture();
    for r in &batch.records {
        assert!(
            r.trace_id().is_some(),
            "every record should join to its trace: {r:?}"
        );
        let id = r.trace_id().unwrap();
        // 32 hex characters: a W3C trace id. A wrong length means the bytes were
        // not hex-encoded somewhere.
        assert_eq!(id.len(), 32, "{id}");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()), "{id}");
    }
}

/// Records from the two exports are two distinct traces, not one merged blob.
#[test]
fn distinct_exports_stay_distinct_traces() {
    let batch = capture();
    let traces: std::collections::BTreeSet<&str> =
        batch.records.iter().filter_map(|r| r.trace_id()).collect();
    assert_eq!(traces.len(), 2, "two exports, two traces: {traces:?}");

    // All four records of one export belong to the same trace, so a trace-scoped
    // log query returns the whole request rather than one line of it.
    let first = traces.iter().next().unwrap();
    let in_first = batch
        .records
        .iter()
        .filter(|r| r.trace_id() == Some(*first))
        .count();
    assert_eq!(in_first, 4, "records sharing a trace: {traces:?}");
}

/// Severities arrive as both a number and a text, and the classification has to
/// come out right for each band.
#[test]
fn severities_cover_every_band() {
    let batch = capture();
    let mut counts = std::collections::BTreeMap::new();
    for r in &batch.records {
        *counts.entry(r.severity().as_str()).or_insert(0) += 1;
    }
    assert_eq!(counts.get("Info"), Some(&2), "{counts:?}");
    assert_eq!(counts.get("Warn"), Some(&2), "{counts:?}");
    assert_eq!(counts.get("Error"), Some(&2), "{counts:?}");
    assert_eq!(counts.get("Debug"), Some(&2), "{counts:?}");

    // The text must be preserved too, since it is what a user reads.
    let error = batch
        .records
        .iter()
        .find(|r| r.severity() == Severity::Error)
        .unwrap();
    assert_eq!(error.severity_text, "Error");
    assert_eq!(error.severity_number, 17);
}

/// Timestamps must be real, not the fixed32 flags field read as a time.
#[test]
fn timestamps_are_real_nanosecond_values() {
    let batch = capture();
    for r in &batch.records {
        // Nanoseconds since the epoch for a 2023 capture: far beyond 2^32, which
        // is exactly the range a fixed32 read would be stuck in.
        assert!(
            r.time_unix_nano > 1_600_000_000_000_000_000,
            "implausible timestamp {} for {:?}",
            r.time_unix_nano,
            r.body
        );
        assert!(r.observed_time_unix_nano >= r.time_unix_nano);
    }
}

/// The structured scope attributes come through, so a query can filter on them
/// instead of parsing the body.
#[test]
fn scope_attributes_are_available_for_filtering() {
    let batch = capture();
    let r = &batch.records[0];
    assert_eq!(
        r.attributes.get("Service"),
        Some(&"agency".to_string()),
        "{r:?}"
    );
    assert_eq!(r.attributes.get("SpanId").is_some(), true, "{r:?}");
}

// ── the store, fed the same capture ──

/// The whole path, on real bytes: ingest, then the three queries an incident
/// actually starts with.
///
/// These are the questions someone asks in order — what blew up, what did the
/// failing request log on its way in, and what else was broken in the same
/// window — so the queries are written as those questions rather than as filter
/// permutations.
#[test]
fn the_capture_answers_the_questions_an_incident_asks() {
    use tinylink::sdg::log::Severity;
    use tinylink::sdg::log_store::{LogQuery, LogStore};

    let mut store = LogStore::temporary("capture").expect("store");
    let report = store
        .insert_bytes(include_bytes!("fixtures/otlp-logs-capture.bin"))
        .expect("ingest");
    assert_eq!(report.inserted, 8, "{report:?}");
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert_eq!(store.services().unwrap(), vec!["Agency.API"]);

    // 1. What blew up?
    let errors = store
        .query(&LogQuery {
            min_severity: Some(Severity::Error),
            limit: 10,
            ..Default::default()
        })
        .expect("query");
    assert_eq!(errors.matches.len(), 2, "{errors:?}");

    // 2. What did the failing request log on its way in? Take one error's trace.
    let failing_trace = errors.matches[0]
        .record
        .trace_id()
        .expect("an error should be traceable")
        .to_string();
    let request = store
        .query(&LogQuery {
            trace_id: Some(failing_trace.clone()),
            limit: 50,
            ..Default::default()
        })
        .expect("query");
    assert_eq!(
        request.matches.len(),
        4,
        "the whole request should come back, not just the error: {request:?}"
    );

    // 3. Now the part a body-only search gets wrong. The interpolated value is
    //    only in an attribute, and that is the string an operator pastes in.
    let by_value = store
        .query(&LogQuery {
            contains: Some("GetAgencyMembers".into()),
            min_severity: Some(Severity::Error),
            limit: 10,
            ..Default::default()
        })
        .expect("query");
    // Both exports logged the same failure, so both come back — which is the point:
    // the search reaches across exports instead of only the one being examined.
    assert_eq!(
        by_value.matches.len(),
        2,
        "an attribute value must be findable by text: {by_value:?}"
    );
    assert!(by_value
        .matches
        .iter()
        .all(|m| m.record.attributes.get("exception.type")
            == Some(&"InvalidOperationException".to_string())));
    assert_eq!(
        by_value.traces().len(),
        2,
        "and they are two distinct traces: {:?}",
        by_value.traces()
    );
}

/// Filter by attribute key, which is how a caller narrows to one structured field
/// such as `exception.type` without knowing its value.
#[test]
fn records_can_be_found_by_attribute_key() {
    use tinylink::sdg::log_store::{LogQuery, LogStore};

    let mut store = LogStore::temporary("attrkey").expect("store");
    store
        .insert_bytes(include_bytes!("fixtures/otlp-logs-capture.bin"))
        .expect("ingest");

    let stacktraces = store
        .query(&LogQuery {
            attribute_key: Some("exception.stacktrace".into()),
            limit: 10,
            ..Default::default()
        })
        .expect("query");
    // Two errors, one per export.
    assert_eq!(stacktraces.matches.len(), 2, "{stacktraces:?}");
    assert!(stacktraces.matches.iter().all(|m| m
        .record
        .attributes
        .get("exception.stacktrace")
        .is_some()));
}

/// A time window that predates the capture must come back empty *and* say the
/// store is not empty, so "no logs in that window" is distinguishable from "no
/// logs at all".
#[test]
fn an_empty_time_window_is_distinguishable_from_an_empty_store() {
    use tinylink::sdg::log_store::{LogQuery, LogStore};

    let mut store = LogStore::temporary("window").expect("store");
    store
        .insert_bytes(include_bytes!("fixtures/otlp-logs-capture.bin"))
        .expect("ingest");

    let out = store
        .query(&LogQuery {
            since_unix_nano: Some(1),
            until_unix_nano: Some(2),
            limit: 10,
            ..Default::default()
        })
        .expect("query");
    assert!(out.matches.is_empty());
    assert_eq!(out.total_matched, 0);
    assert_eq!(out.scanned, 8, "the store does hold records");
    assert_eq!(out.untimed, 0, "every record in the capture is timestamped");
}
