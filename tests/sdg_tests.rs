fn fixture_path(parts: &[&str]) -> String {
    let base = std::env::current_dir().unwrap();
    let mut path = base.join("tests").join("fixtures");
    for p in parts {
        path = path.join(p);
    }
    path.to_string_lossy().to_string()
}

fn read_fixture(parts: &[&str]) -> String {
    std::fs::read_to_string(fixture_path(parts)).expect("fixture not found")
}

#[test]
fn test_parse_simple_request() {
    let json = read_fixture(&["otel", "simple_request.json"]);
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    assert_eq!(graph.total_traces, 1);
    assert_eq!(graph.error_traces, 0);
    assert!(graph.nodes.contains_key("Frontend"));
    assert!(graph.nodes.contains_key("CatalogAPI"));
    assert!(graph.nodes.contains_key("OrderingAPI"));
    assert!(graph.nodes.contains_key("orders_db"));
    assert_eq!(graph.nodes.len(), 4);
    assert!(!graph.edges.is_empty());
}

#[test]
fn test_parse_error_request() {
    let json = read_fixture(&["otel", "error_request.json"]);
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    assert_eq!(graph.error_traces, 1);
    let has_error_edge = graph.edges.iter().any(|e| e.error_count > 0);
    assert!(has_error_edge);
}

#[test]
fn test_parse_multi_trace() {
    let json = read_fixture(&["otel", "multi_trace.json"]);
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    assert_eq!(graph.total_traces, 2);
    assert!(graph.nodes.len() >= 3);
    assert!(graph.nodes.contains_key("Gateway"));
    assert!(graph.nodes.contains_key("SearchAPI"));
    assert!(graph.nodes.contains_key("CatalogAPI"));
    assert!(!graph.edges.is_empty());
}

#[test]
fn test_sdg_to_dot_output() {
    let json = read_fixture(&["otel", "simple_request.json"]);
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    let dot = graph.to_dot();
    assert!(dot.starts_with("digraph SDG"));
    assert!(dot.contains("Frontend"));
    assert!(dot.ends_with("}\n"));
}

#[test]
fn test_sdg_stats_match() {
    let json = read_fixture(&["otel", "multi_trace.json"]);
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    assert_eq!(graph.total_traces, 2);
    assert_eq!(graph.error_traces, 0);
}

#[test]
fn test_parse_invalid_json() {
    let result = tinylink::sdg::otel::parse_trace_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_file() {
    let result = tinylink::sdg::otel::parse_trace_json("{}");
    // Should fail because resourceSpans is missing
    assert!(result.is_err());
}
