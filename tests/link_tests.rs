fn fixture_path(parts: &[&str]) -> String {
    let base = std::env::current_dir().unwrap();
    let mut path = base.join("tests").join("fixtures");
    for p in parts {
        path = path.join(p);
    }
    path.to_string_lossy().to_string()
}

#[test]
fn test_sdg_roundtrip_json() {
    let json = std::fs::read_to_string(fixture_path(&["otel", "simple_request.json"]))
        .expect("fixture not found");
    let graph = tinylink::sdg::otel::parse_trace_json(&json).unwrap();
    let serialized = serde_json::to_string_pretty(&graph).unwrap();
    let deserialized: tinylink::sdg::types::ServiceGraph =
        serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.nodes.len(), graph.nodes.len());
    assert_eq!(deserialized.edges.len(), graph.edges.len());
}

#[test]
fn test_route_table_basic_matching() {
    let mut table = tinylink::code::route::RouteTable::new();
    table.add(tinylink::code::route::RouteEntry {
        http_method: "GET".into(),
        route_pattern: "/api/items/{id}".into(),
        class: "ItemsController".into(),
        method: "GetItem".into(),
    });
    let matches = table.match_route("GET", "/api/items/42");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class, "ItemsController");
}

#[test]
fn test_unified_graph_serialization() {
    use tinylink::link::types::UnifiedGraph;
    let mut g = UnifiedGraph::new();
    g.add_service_node("SvcA", 0);
    g.add_service_node("SvcB", 1);
    let json = serde_json::to_string_pretty(&g).unwrap();
    assert!(json.contains("SvcA"));
    assert!(json.contains("SvcB"));
}
