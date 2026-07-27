use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_small_trace() {
    let json = r#"{
        "resourceSpans": [{
            "resource": { "attributes": [{"key": "service.name", "value": {"stringValue": "Svc"}}] },
            "scopeSpans": [{
                "spans": [{
                    "traceId": "t1", "spanId": "s1", "parentSpanId": "",
                    "name": "GET /api", "kind": 2,
                    "startTimeUnixNano": 0, "endTimeUnixNano": 100000000
                }, {
                    "traceId": "t1", "spanId": "s2", "parentSpanId": "s1",
                    "name": "GET /api", "kind": 3,
                    "attributes": [{"key": "http.method", "value": {"stringValue": "GET"}},
                                   {"key": "http.target", "value": {"stringValue": "/api/items"}}],
                    "startTimeUnixNano": 0, "endTimeUnixNano": 50000000
                }]
            }]
        }, {
            "resource": { "attributes": [{"key": "service.name", "value": {"stringValue": "Backend"}}] },
            "scopeSpans": [{
                "spans": [{
                    "traceId": "t1", "spanId": "s3", "parentSpanId": "s2",
                    "name": "POST /process", "kind": 2,
                    "attributes": [{"key": "http.method", "value": {"stringValue": "POST"}},
                                   {"key": "http.route", "value": {"stringValue": "/process"}}],
                    "startTimeUnixNano": 50000000, "endTimeUnixNano": 100000000
                }]
            }]
        }]
    }"#;

    let _ = tinylink::sdg::otel::parse_trace_json(black_box(json)).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("sdg_parse_small_trace", |b| {
        b.iter(bench_parse_small_trace)
    });

    c.bench_function("sdg_parse_empty_trace", |b| {
        b.iter(|| {
            let json = r#"{"resourceSpans": []}"#;
            let _ = tinylink::sdg::otel::parse_trace_json(json).unwrap();
        })
    });

    c.bench_function("service_graph_add_edge", |b| {
        let mut graph = tinylink::sdg::types::ServiceGraph::new();
        b.iter(|| {
            graph.add_edge("A", "B", "ep1", "ep2", false, 100.0, "trace1");
        })
    });

    c.bench_function("service_graph_to_dot", |b| {
        let mut graph = tinylink::sdg::types::ServiceGraph::new();
        for i in 0..10 {
            graph.add_edge(
                &format!("Svc{}", i),
                &format!("Svc{}", i + 1),
                "ep1", "ep2", false, 50.0, "t",
            );
        }
        b.iter(|| {
            let _ = black_box(graph.to_dot());
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
