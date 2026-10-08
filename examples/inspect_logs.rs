//! Parse a real OTLP log capture.
//!
//! `tests/fixtures/otlp-logs-capture.bin` is raw OTLP/protobuf produced by the
//! OpenTelemetry .NET SDK 1.18.0 — captured by running an actual application
//! against a listener and dumping the request bytes. Nothing here is hand-written
//! protobuf, so the field numbers, the wire types and the attribute encoding are
//! whatever the SDK actually does.
//!
//! It is a fixture rather than a mock because the failure mode this guards against
//! is precisely a wrong field number: a hand-rolled payload would have been built
//! to match the parser, so it could not catch the parser being wrong.
//!
//! Run with: cargo run --example inspect_logs

use tinylink::sdg::log_otlp::parse_otlp_http;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/otlp-logs-capture.bin"
        )
        .to_string()
    });
    let bytes = std::fs::read(&path).expect("capture file");

    let batch = parse_otlp_http(&bytes, None).expect("parse");

    println!("service.name      : {:?}", batch.service_name());
    println!("resource attrs    : {}", batch.resource_attributes.len());
    for (k, v) in &batch.resource_attributes {
        println!("    {k} = {v}");
    }
    println!("records           : {}", batch.records.len());

    let mut with_trace = 0;
    for r in &batch.records {
        if r.trace_id().is_some() {
            with_trace += 1;
        }
        println!(
            "  [{}] trace={} span={} attrs={}",
            r.severity().as_str(),
            r.trace_id().unwrap_or("-"),
            r.span_id().unwrap_or("-"),
            r.attributes.len()
        );
        println!("       body: {}", r.body);
        for (k, v) in &r.attributes {
            println!("         {k} = {v}");
        }
    }
    println!(
        "records carrying a trace id: {with_trace}/{}",
        batch.records.len()
    );

    let mut by_severity = std::collections::BTreeMap::new();
    for r in &batch.records {
        *by_severity.entry(r.severity().as_str()).or_insert(0) += 1;
    }
    println!("by severity       : {by_severity:?}");
}
