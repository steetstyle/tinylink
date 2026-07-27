use std::collections::HashMap;
use std::fs;
use serde::Deserialize;
use anyhow::{Context, Result};

use super::types::ServiceGraph;

/// Minimal OTLP JSON span model — enough to build SDG
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelSpan {
    pub trace_id: String,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: Option<i32>,
    pub status: Option<OtelStatus>,
    pub attributes: Option<Vec<OtelAttribute>>,
    #[serde(default)]
    pub start_time_unix_nano: u64,
    #[serde(default)]
    pub end_time_unix_nano: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelStatus {
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OtelAttribute {
    pub key: String,
    pub value: OtelAttributeValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OtelAttributeValue {
    StringValue { #[serde(rename = "stringValue")] string_value: String },
    IntValue { #[serde(rename = "intValue")] int_value: i64 },
    DoubleValue { #[serde(rename = "doubleValue")] double_value: f64 },
    BoolValue { #[serde(rename = "boolValue")] bool_value: bool },
}

/// OTLP JSON resource spans structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelResourceSpans {
    pub resource: Option<OtelResource>,
    pub scope_spans: Option<Vec<OtelScopeSpans>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelResource {
    pub attributes: Option<Vec<OtelAttribute>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelScopeSpans {
    pub spans: Vec<OtelSpan>,
}

/// Top-level OTLP export structure
#[derive(Debug, Deserialize)]
pub struct OtelExportTrace {
    #[serde(alias = "resourceSpans")]
    pub resource_spans: Vec<OtelResourceSpans>,
}

/// Span kind constants matching otel protobuf
pub const SPAN_KIND_CLIENT: i32 = 3;
pub const SPAN_KIND_SERVER: i32 = 2;
pub const SPAN_KIND_PRODUCER: i32 = 4;
pub const SPAN_KIND_CONSUMER: i32 = 5;
pub const SPAN_KIND_INTERNAL: i32 = 1;

/// Error status code
pub const STATUS_CODE_ERROR: i32 = 2;

/// Parse OTLP JSON trace file and build ServiceGraph
pub fn parse_trace_file(path: &str) -> Result<ServiceGraph> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read trace file: {}", path))?;
    parse_trace_json(&content)
}

/// Parse OTLP JSON string content and build ServiceGraph
pub fn parse_trace_json(content: &str) -> Result<ServiceGraph> {
    let export: OtelExportTrace = serde_json::from_str(content)
        .context("Failed to parse OTLP JSON")?;

    let mut graph = ServiceGraph::new();
    let mut spans_by_trace: HashMap<String, Vec<SpanEnriched>> = HashMap::new();

    for rspans in &export.resource_spans {
        let resource_attrs = rspans.resource.as_ref()
            .and_then(|r| r.attributes.as_ref());

        let scope_spans = match &rspans.scope_spans {
            Some(s) => s,
            None => continue,
        };

        for scope in scope_spans {
            for span in &scope.spans {
                let service_name = resource_attrs
                    .and_then(|attrs| get_attr(attrs, "service.name"))
                    .unwrap_or_else(|| "unknown".to_string());

                let enriched = SpanEnriched {
                    trace_id: span.trace_id.clone(),
                    span_id: span.span_id.clone().unwrap_or_default(),
                    parent_span_id: span.parent_span_id.clone().unwrap_or_default(),
                    name: span.name.clone(),
                    kind: span.kind.unwrap_or(SPAN_KIND_INTERNAL),
                    service_name,
                    status_code: span.status.as_ref().map(|s| s.code.unwrap_or(0)).unwrap_or(0),
                    attributes: span.attributes.clone().unwrap_or_default(),
                    start_time_nano: span.start_time_unix_nano,
                    end_time_nano: span.end_time_unix_nano,
                };

                spans_by_trace.entry(enriched.trace_id.clone())
                    .or_default()
                    .push(enriched);
            }
        }
    }

    graph.total_traces = spans_by_trace.len();

    for (trace_id, spans) in &spans_by_trace {
        let has_error = spans.iter().any(|s| s.status_code == STATUS_CODE_ERROR);
        if has_error {
            graph.error_traces += 1;
        }
        process_trace_spans(&mut graph, trace_id, &spans);
    }

    Ok(graph)
}

#[derive(Debug, Clone)]
struct SpanEnriched {
    trace_id: String,
    span_id: String,
    parent_span_id: String,
    name: String,
    kind: i32,
    service_name: String,
    status_code: i32,
    attributes: Vec<OtelAttribute>,
    start_time_nano: u64,
    end_time_nano: u64,
}

fn process_trace_spans(graph: &mut ServiceGraph, trace_id: &str, spans: &[SpanEnriched]) {
    let by_kind: HashMap<i32, Vec<&SpanEnriched>> = spans.iter()
        .filter(|s| s.kind == SPAN_KIND_CLIENT || s.kind == SPAN_KIND_SERVER
            || s.kind == SPAN_KIND_PRODUCER || s.kind == SPAN_KIND_CONSUMER)
        .fold(HashMap::new(), |mut acc, s| {
            acc.entry(s.kind).or_default().push(s);
            acc
        });

    // Build parent→children map
    let parent_map: HashMap<&str, Vec<&SpanEnriched>> = spans.iter()
        .filter(|s| !s.parent_span_id.is_empty())
        .fold(HashMap::new(), |mut acc, s| {
            acc.entry(s.parent_span_id.as_str()).or_default().push(s);
            acc
        });

    // Process client → server pairs
    if let Some(clients) = by_kind.get(&SPAN_KIND_CLIENT) {
        for client in clients {
            let source_ep = extract_endpoint(client);
            let error = is_error_span(client);
            let latency = duration_ms(client.start_time_nano, client.end_time_nano);

            // Find matching server span: server.parent_span_id == client.span_id
            let server = by_kind.get(&SPAN_KIND_SERVER)
                .and_then(|servers| {
                    servers.iter().find(|s| s.parent_span_id == client.span_id)
                })
                .or_else(|| {
                    let client_id = client.span_id.as_str();
                    parent_map.get(client_id)
                        .and_then(|children| {
                            children.iter().find(|c| c.kind == SPAN_KIND_SERVER)
                        })
                });

            match server {
                Some(server) => {
                    let target_ep = extract_endpoint(server);
                    graph.add_edge(
                        &client.service_name,
                        &server.service_name,
                        &source_ep,
                        &target_ep,
                        error,
                        latency,
                        trace_id,
                    );
                }
                None => {
                    // Check for database or messaging
                    let is_db = client.attributes.iter().any(|a| {
                        a.key == "db.system" || a.key == "db.name" || a.key == "db.namespace"
                    });
                    let is_messaging = client.attributes.iter().any(|a| {
                        a.key.starts_with("messaging.")
                    });

                    if is_db {
                        let db_name = get_attr(&client.attributes, "db.name")
                            .or_else(|| get_attr(&client.attributes, "db.system"))
                            .unwrap_or_else(|| "database".to_string());
                        graph.add_edge(
                            &client.service_name,
                            &db_name,
                            &source_ep,
                            &source_ep,
                            error,
                            latency,
                            trace_id,
                        );
                    } else if is_messaging {
                        let queue = get_attr(&client.attributes, "messaging.destination")
                            .unwrap_or_else(|| "queue".to_string());
                        graph.add_edge(
                            &client.service_name,
                            &queue,
                            &source_ep,
                            &source_ep,
                            error,
                            latency,
                            trace_id,
                        );
                    }
                    // else: missing server span — skip (virtual node case)
                }
            }
        }
    }

    // Process producer → consumer pairs
    if let Some(producers) = by_kind.get(&SPAN_KIND_PRODUCER) {
        let consumers = by_kind.get(&SPAN_KIND_CONSUMER).cloned().unwrap_or_default();
        for producer in producers {
            let source_ep = extract_endpoint(producer);
            let error = is_error_span(producer);
            let latency = duration_ms(producer.start_time_nano, producer.end_time_nano);
            let consumer = consumers.iter().find(|c| c.parent_span_id == producer.span_id);

            if let Some(consumer) = consumer {
                let target_ep = extract_endpoint(consumer);
                graph.add_edge(
                    &producer.service_name,
                    &consumer.service_name,
                    &source_ep,
                    &target_ep,
                    error,
                    latency,
                    trace_id,
                );
            }
        }
    }
}

fn extract_endpoint(span: &SpanEnriched) -> String {
    // gRPC: rpc.service/rpc.method → "POST /{rpc.service}/{rpc.method}"
    let rpc_service = get_attr(&span.attributes, "rpc.service");
    let rpc_method = get_attr(&span.attributes, "rpc.method");
    if let (Some(ref svc), Some(ref method)) = (rpc_service, rpc_method) {
        return format!("POST /{}/{}", svc, method);
    }

    // HTTP-style
    let http_method = get_attr(&span.attributes, "http.method")
        .unwrap_or_else(|| "GET".to_string());
    let route = get_attr(&span.attributes, "http.route")
        .or_else(|| get_attr(&span.attributes, "http.target"))
        .unwrap_or_else(|| span.name.clone());
    format!("{} {}", http_method, route)
}

fn get_attr(attrs: &[OtelAttribute], key: &str) -> Option<String> {
    attrs.iter().find(|a| a.key == key).and_then(|a| match &a.value {
        OtelAttributeValue::StringValue { string_value } => Some(string_value.clone()),
        OtelAttributeValue::IntValue { int_value } => Some(int_value.to_string()),
        OtelAttributeValue::DoubleValue { double_value } => Some(double_value.to_string()),
        OtelAttributeValue::BoolValue { bool_value } => Some(bool_value.to_string()),
    })
}

fn is_error_span(span: &SpanEnriched) -> bool {
    if span.status_code == STATUS_CODE_ERROR {
        return true;
    }
    // http.status_code attribute (set by HTTP instrumentations)
    if let Some(code_str) = get_attr(&span.attributes, "http.status_code") {
        if let Ok(code) = code_str.parse::<i32>() {
            if code >= 400 {
                return true;
            }
        }
    }
    false
}

fn duration_ms(start_nano: u64, end_nano: u64) -> f64 {
    if end_nano > start_nano {
        (end_nano - start_nano) as f64 / 1_000_000.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_trace() {
        let json = r#"{"resourceSpans": []}"#;
        let graph = parse_trace_json(json).unwrap();
        assert_eq!(graph.total_traces, 0);
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn test_parse_single_request() {
        let json = r#"{
            "resourceSpans": [{
                "resource": { "attributes": [{"key": "service.name", "value": {"stringValue": "Frontend"}}] },
                "scopeSpans": [{
                    "spans": [
                        {
                            "traceId": "abc",
                            "spanId": "span1",
                            "parentSpanId": "",
                            "name": "GET /api",
                            "kind": 2,
                            "startTimeUnixNano": 0,
                            "endTimeUnixNano": 100000000
                        },
                        {
                            "traceId": "abc",
                            "spanId": "span2",
                            "parentSpanId": "span1",
                            "name": "GET /api",
                            "kind": 3,
                            "status": {"code": 2},
                            "attributes": [
                                {"key": "http.method", "value": {"stringValue": "GET"}},
                                {"key": "http.target", "value": {"stringValue": "/api/items"}}
                            ],
                            "startTimeUnixNano": 0,
                            "endTimeUnixNano": 50000000
                        }
                    ]
                }]
            }, {
                "resource": { "attributes": [{"key": "service.name", "value": {"stringValue": "Backend"}}] },
                "scopeSpans": [{
                    "spans": [{
                        "traceId": "abc",
                        "spanId": "span3",
                        "parentSpanId": "span2",
                        "name": "POST /process",
                        "kind": 2,
                        "attributes": [
                            {"key": "http.method", "value": {"stringValue": "POST"}},
                            {"key": "http.route", "value": {"stringValue": "/process"}}
                        ],
                        "startTimeUnixNano": 50000000,
                        "endTimeUnixNano": 100000000
                    }]
                }]
            }]
        }"#;
        let graph = parse_trace_json(json).unwrap();
        assert_eq!(graph.total_traces, 1);
        assert_eq!(graph.error_traces, 1);
        assert!(graph.nodes.contains_key("Frontend"));
        assert!(graph.nodes.contains_key("Backend"));
        assert!(!graph.edges.is_empty());
        // Client span from Frontend, server = Backend
        let frontend_edge = graph.edges.iter().find(|e| e.source == "Frontend");
        assert!(frontend_edge.is_some(), "expected edge from Frontend");
        if let Some(e) = frontend_edge {
            assert_eq!(e.target, "Backend");
            assert!(e.error_count > 0);
        }
    }

    #[test]
    fn test_parse_db_request() {
        let json = r#"{
            "resourceSpans": [{
                "resource": { "attributes": [{"key": "service.name", "value": {"stringValue": "App"}}] },
                "scopeSpans": [{
                    "spans": [{
                        "traceId": "t1",
                        "spanId": "s1",
                        "parentSpanId": "",
                        "name": "SELECT",
                        "kind": 3,
                        "attributes": [
                            {"key": "db.system", "value": {"stringValue": "postgresql"}},
                            {"key": "db.name", "value": {"stringValue": "orders_db"}}
                        ],
                        "startTimeUnixNano": 0,
                        "endTimeUnixNano": 200000000
                    }]
                }]
            }]
        }"#;
        let graph = parse_trace_json(json).unwrap();
        assert!(graph.nodes.contains_key("App"));
        assert!(graph.nodes.contains_key("orders_db"));
        let db_edge = graph.edges.iter().find(|e| e.target == "orders_db");
        assert!(db_edge.is_some());
    }

    #[test]
    fn test_extract_endpoint_with_route() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "test".into(),
            kind: SPAN_KIND_SERVER,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![
                OtelAttribute {
                    key: "http.method".into(),
                    value: OtelAttributeValue::StringValue { string_value: "POST".into() },
                },
                OtelAttribute {
                    key: "http.route".into(),
                    value: OtelAttributeValue::StringValue { string_value: "/api/items/{id}".into() },
                },
            ],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert_eq!(extract_endpoint(&span), "POST /api/items/{id}");
    }

    #[test]
    fn test_extract_endpoint_fallback_to_name() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "grpc.Call".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert_eq!(extract_endpoint(&span), "GET grpc.Call");
    }

    #[test]
    fn test_duration_ms() {
        assert!((duration_ms(0, 10_000_000) - 10.0).abs() < 0.001);
        assert_eq!(duration_ms(0, 0), 0.0);
        assert_eq!(duration_ms(100, 50), 0.0);
    }

    #[test]
    fn test_grpc_endpoint() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "grpc.Call".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![
                OtelAttribute {
                    key: "rpc.service".into(),
                    value: OtelAttributeValue::StringValue { string_value: "package.GreeterService".into() },
                },
                OtelAttribute {
                    key: "rpc.method".into(),
                    value: OtelAttributeValue::StringValue { string_value: "SayHello".into() },
                },
            ],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert_eq!(extract_endpoint(&span), "POST /package.GreeterService/SayHello");
    }

    #[test]
    fn test_grpc_endpoint_missing_service() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "grpc.Call".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![
                OtelAttribute {
                    key: "rpc.method".into(),
                    value: OtelAttributeValue::StringValue { string_value: "SayHello".into() },
                },
            ],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        // No rpc.service → falls through to HTTP-style
        assert_eq!(extract_endpoint(&span), "GET grpc.Call");
    }

    #[test]
    fn test_http_status_code_error() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "GET /fail".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![
                OtelAttribute {
                    key: "http.status_code".into(),
                    value: OtelAttributeValue::StringValue { string_value: "500".into() },
                },
            ],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert!(is_error_span(&span));
    }

    #[test]
    fn test_http_status_code_ok() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "GET /ok".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 0,
            attributes: vec![
                OtelAttribute {
                    key: "http.status_code".into(),
                    value: OtelAttributeValue::StringValue { string_value: "200".into() },
                },
            ],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert!(!is_error_span(&span));
    }

    #[test]
    fn test_is_error_span_status_code() {
        let span = SpanEnriched {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: "".into(),
            name: "call".into(),
            kind: SPAN_KIND_CLIENT,
            service_name: "svc".into(),
            status_code: 2,
            attributes: vec![],
            start_time_nano: 0,
            end_time_nano: 100_000_000,
        };
        assert!(is_error_span(&span));
    }
}
