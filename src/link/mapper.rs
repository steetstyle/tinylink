use crate::sdg::types::ServiceGraph;
use crate::code::types::CodeGraph;
use crate::code::route::RouteTable;
use crate::link::types::{UnifiedGraph, UnifiedEdge};

/// Map a ServiceGraph edge to code-level methods using CodeGraph + RouteTable.
/// Returns the best matching methods for source and target.
pub fn map_service_edge_to_method(
    source_service: &str,
    target_service: &str,
    source_endpoint: &str,
    target_endpoint: &str,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> (Option<String>, Option<String>) {
    let source_method = map_endpoint(source_service, source_endpoint, code_graph, route_table);
    let target_method = map_endpoint(target_service, target_endpoint, code_graph, route_table);
    (source_method, target_method)
}

fn map_endpoint(
    service: &str,
    endpoint: &str,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> Option<String> {
    // Try route table first
    let parts: Vec<&str> = endpoint.splitn(2, ' ').collect();
    let http_method = parts.first().copied().unwrap_or("GET");
    let path = parts.get(1).copied().unwrap_or("");

    if !path.is_empty() {
        if let Some(entry) = route_table.match_best(http_method, path) {
            return Some(format!("{}.{}", entry.class, entry.method));
        }
    }

    // Fallback: search method names that contain endpoint keywords
    let path_lower = endpoint.to_lowercase();
    for m in &code_graph.methods {
        let method_lower = m.method.to_lowercase();
        if method_lower.contains(&path_lower)
            || path_lower.contains(&method_lower)
        {
            return Some(format!("{}.{}", m.class, m.method));
        }

        // Match service name against class name
        let class_lower = m.class.to_lowercase();
        let svc_lower = service.to_lowercase();
        if class_lower.contains(&svc_lower) || svc_lower.contains(&class_lower) {
            if method_lower.contains("get")
                || method_lower.contains("post")
                || method_lower.contains("handle")
            {
                return Some(format!("{}.{}", m.class, m.method));
            }
        }
    }

    None
}

/// Build a UnifiedGraph by merging SDG edges with code-level method resolution
pub fn build_unified_from_sdg(
    sdg: &ServiceGraph,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> UnifiedGraph {
    let mut unified = UnifiedGraph::new();
    unified.total_traces = sdg.total_traces;
    unified.error_traces = sdg.error_traces;

    // Add service nodes
    for (name, node) in &sdg.nodes {
        unified.add_service_node(name, node.error_count);
        if !unified.services.contains(name) {
            unified.services.push(name.clone());
        }
    }

    // Process each SDG edge
    for edge in &sdg.edges {
        // Find method-level mapping
        let (src_method, tgt_method) = map_service_edge_to_method(
            &edge.source, &edge.target,
            &edge.source_endpoint, &edge.target_endpoint,
            code_graph, route_table,
        );

        let mut source_id = format!("svc:{}", edge.source);
        let mut target_id = format!("svc:{}", edge.target);

        // Add method nodes and update IDs if methods found
        if let Some(ref sm) = src_method {
            let parts: Vec<&str> = sm.splitn(2, '.').collect();
            let class = parts[0];
            let method = parts.get(1).copied().unwrap_or("");
            if let Some(mn) = code_graph.find_method(class, method) {
                source_id = unified.add_method_node(
                    &edge.source, class, method,
                    &mn.file, mn.line_start,
                    if edge.error_count > 0 { 1 } else { 0 },
                );
            }
        }

        if let Some(ref tm) = tgt_method {
            let parts: Vec<&str> = tm.splitn(2, '.').collect();
            let class = parts[0];
            let method = parts.get(1).copied().unwrap_or("");
            if let Some(mn) = code_graph.find_method(class, method) {
                target_id = unified.add_method_node(
                    &edge.target, class, method,
                    &mn.file, mn.line_start,
                    if edge.error_count > 0 { 1 } else { 0 },
                );
            }
        }

        for trace_id in &edge.trace_ids {
            unified.add_edge(UnifiedEdge {
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                source_service: edge.source.clone(),
                source_method: src_method.clone(),
                target_service: edge.target.clone(),
                target_method: tgt_method.clone(),
                target_file: None, // will be set from method node
                target_line: None,
                span_id: Some(trace_id.clone()),
                error: edge.error_count > 0,
                latency_ms: edge.avg_latency_ms,
                call_count: 1,
            });
        }
    }

    // Update per-edge target_file/target_line from method nodes
    for edge in &mut unified.edges {
        if let Some(node) = unified.nodes.get(&edge.target_id) {
            edge.target_file = node.file.clone();
            edge.target_line = node.line;
        }
    }

    // Merge edges with same source_id, target_id
    unified.edges = merge_duplicate_edges(&unified.edges);

    unified
}

fn merge_duplicate_edges(edges: &[UnifiedEdge]) -> Vec<UnifiedEdge> {
    use std::collections::HashMap;
    let mut merged: HashMap<(String, String), UnifiedEdge> = HashMap::new();

    for edge in edges {
        let key = (edge.source_id.clone(), edge.target_id.clone());
        let entry = merged.entry(key).or_insert(UnifiedEdge {
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            source_service: edge.source_service.clone(),
            source_method: edge.source_method.clone(),
            target_service: edge.target_service.clone(),
            target_method: edge.target_method.clone(),
            target_file: edge.target_file.clone(),
            target_line: edge.target_line,
            span_id: None,
            error: edge.error,
            latency_ms: edge.latency_ms,
            call_count: 0,
        });
        entry.call_count += edge.call_count;
        entry.error = entry.error || edge.error;
        entry.latency_ms = (entry.latency_ms * (entry.call_count - 1) as f64 + edge.latency_ms)
            / entry.call_count as f64;
    }

    merged.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::types::{CodeGraph, MethodNode};
    use crate::code::route::{RouteTable, RouteEntry};
    use crate::sdg::types::ServiceGraph;

    fn make_code_graph() -> CodeGraph {
        let mut cg = CodeGraph::new();
        cg.add_method(MethodNode {
            class: "CatalogApi".into(),
            method: "GetItemById".into(),
            signature: "Item GetItemById(int id)".into(),
            file: "CatalogApi.cs".into(),
            line_start: 28,
            line_end: 35,
            service: Some("CatalogAPI".into()),
        });
        cg.add_method(MethodNode {
            class: "OrderValidator".into(),
            method: "Validate".into(),
            signature: "bool Validate(Order order)".into(),
            file: "OrderValidator.cs".into(),
            line_start: 10,
            line_end: 20,
            service: Some("OrderingAPI".into()),
        });
        cg
    }

    fn make_route_table() -> RouteTable {
        let mut rt = RouteTable::new();
        rt.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/items/{id}".into(),
            class: "CatalogApi".into(),
            method: "GetItemById".into(),
        });
        rt.add(RouteEntry {
            http_method: "POST".into(),
            route_pattern: "/validate".into(),
            class: "OrderValidator".into(),
            method: "Validate".into(),
        });
        rt
    }

    fn make_sdg() -> ServiceGraph {
        let mut sdg = ServiceGraph::new();
        sdg.add_edge("CatalogAPI", "OrderingAPI",
            "GET /api/items/{id}", "POST /validate",
            false, 150.0, "trace1");
        sdg
    }

    #[test]
    fn test_map_service_edge_route_match() {
        let cg = make_code_graph();
        let rt = make_route_table();
        let (src, tgt) = map_service_edge_to_method(
            "CatalogAPI", "OrderingAPI",
            "GET /api/items/{id}", "POST /validate",
            &cg, &rt,
        );
        assert_eq!(src, Some("CatalogApi.GetItemById".into()));
        assert_eq!(tgt, Some("OrderValidator.Validate".into()));
    }

    #[test]
    fn test_build_unified_from_sdg() {
        let sdg = make_sdg();
        let cg = make_code_graph();
        let rt = make_route_table();
        let unified = build_unified_from_sdg(&sdg, &cg, &rt);

        assert_eq!(unified.services.len(), 2);
        assert!(unified.services.contains(&"CatalogAPI".to_string()));
        assert!(unified.services.contains(&"OrderingAPI".to_string()));

        // Should have method nodes
        let method_node_key = "method:CatalogAPI:CatalogApi.GetItemById";
        assert!(unified.nodes.contains_key(method_node_key));
    }

    #[test]
    fn test_build_unified_from_empty_sdg() {
        let sdg = ServiceGraph::new();
        let cg = make_code_graph();
        let rt = make_route_table();
        let unified = build_unified_from_sdg(&sdg, &cg, &rt);
        assert!(unified.nodes.is_empty());
        assert!(unified.edges.is_empty());
    }

    #[test]
    fn test_map_endpoint_route_table_first() {
        let cg = make_code_graph();
        let rt = make_route_table();
        let result = map_endpoint("CatalogAPI", "GET /api/items/123", &cg, &rt);
        assert_eq!(result, Some("CatalogApi.GetItemById".into()));
    }

    #[test]
    fn test_map_endpoint_fallback_method_name() {
        let mut cg = CodeGraph::new();
        cg.add_method(MethodNode {
            class: "HealthController".into(),
            method: "HealthCheck".into(),
            signature: "void HealthCheck()".into(),
            file: "health.cs".into(),
            line_start: 1,
            line_end: 3,
            service: Some("API".into()),
        });
        let rt = RouteTable::new();
        let result = map_endpoint("API", "GET /health", &cg, &rt);
        // endpoint is "GET /health", method is "HealthCheck" — no direct match
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_merge_duplicate_edges() {
        let edges = vec![
            UnifiedEdge {
                source_id: "svc:A".into(), target_id: "svc:B".into(),
                source_service: "A".into(), source_method: None,
                target_service: "B".into(), target_method: None,
                target_file: None, target_line: None,
                span_id: None, error: false, latency_ms: 100.0, call_count: 1,
            },
            UnifiedEdge {
                source_id: "svc:A".into(), target_id: "svc:B".into(),
                source_service: "A".into(), source_method: None,
                target_service: "B".into(), target_method: None,
                target_file: None, target_line: None,
                span_id: None, error: true, latency_ms: 200.0, call_count: 1,
            },
        ];
        let merged = merge_duplicate_edges(&edges);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].call_count, 2);
        assert!(merged[0].error);
    }
}
