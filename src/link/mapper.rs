use crate::code::route::RouteTable;
use crate::code::types::CodeGraph;
use crate::link::types::{UnifiedEdge, UnifiedGraph};
use crate::sdg::types::ServiceGraph;

/// A method resolved for one endpoint, with the confidence of the strategy
/// that produced it. `None` confidence means nothing matched.
pub type MethodMatch = (Option<String>, Option<f64>);

/// Map a ServiceGraph edge to code-level methods using CodeGraph + RouteTable.
/// Returns the source and target matches.
pub fn map_service_edge_to_method(
    source_service: &str,
    target_service: &str,
    source_endpoint: &str,
    target_endpoint: &str,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> (MethodMatch, MethodMatch) {
    let (src_method, src_conf) =
        map_endpoint(source_service, source_endpoint, code_graph, route_table);
    let (tgt_method, tgt_conf) =
        map_endpoint(target_service, target_endpoint, code_graph, route_table);
    ((src_method, src_conf), (tgt_method, tgt_conf))
}

/// Mapping strategies and their confidence scores
fn map_endpoint(
    service: &str,
    endpoint: &str,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> (Option<String>, Option<f64>) {
    // Try route table first (confidence: 0.95)
    let parts: Vec<&str> = endpoint.splitn(2, ' ').collect();
    let http_method = parts.first().copied().unwrap_or("GET");
    let path = parts.get(1).copied().unwrap_or("");

    if !path.is_empty() {
        // Scope the route lookup to this service before trusting it. The route
        // table merges every loaded project, so an unscoped `match_best`
        // returned whichever service happened to register a matching pattern —
        // previously at 0.95, the highest confidence in the scale. Collect all
        // pattern matches first, since the most specific one may belong to a
        // different service than the best in-service one.
        let mut in_service: Option<&crate::code::route::RouteEntry> = None;
        let mut any_match: Option<&crate::code::route::RouteEntry> = None;
        for entry in route_table.match_route(http_method, path) {
            let is_registration = is_registration_method(&entry.method);
            if !is_registration && route_class_belongs_to_service(entry, service, code_graph) {
                in_service = Some(pick_more_specific(in_service, entry));
            } else if !is_registration {
                any_match = Some(pick_more_specific(any_match, entry));
            }
        }
        if let Some(entry) = in_service {
            return (
                Some(format!("{}.{}", entry.class, entry.method)),
                Some(0.95),
            );
        }
        // No in-service route matched. A cross-service pattern match is better
        // than nothing, but it is a weaker claim, so score it below the scoped
        // case rather than presenting it as equally certain.
        if let Some(entry) = any_match {
            return (
                Some(format!("{}.{}", entry.class, entry.method)),
                Some(0.70),
            );
        }
    }

    // gRPC endpoint: "POST /service/Method" → try class.Method directly (confidence: 0.85)
    if let Some(grpc) = try_grpc_method(endpoint, code_graph) {
        return (Some(grpc), Some(0.85));
    }

    // Fallback: keyword match last path segment against method name (confidence: 0.60)
    let last_segment = path.rsplit('/').next().unwrap_or("").to_lowercase();
    let svc_lower = service.to_lowercase();
    for m in &code_graph.methods {
        if !is_handler_class(&m.class) {
            continue;
        }
        if m.class == m.method {
            continue;
        }
        if is_registration_method(&m.method) {
            continue;
        }
        if !method_belongs_to_service(m, &svc_lower) {
            continue;
        }
        let method_lower = m.method.to_lowercase();
        if last_segment.len() > 2
            && (method_lower.contains(&last_segment) || last_segment.contains(&method_lower))
        {
            return (Some(format!("{}.{}", m.class, m.method)), Some(0.60));
        }
    }

    // Last resort: match service name against class name (confidence: 0.40)
    for m in &code_graph.methods {
        if !is_handler_class(&m.class) {
            continue;
        }
        if m.class == m.method {
            continue;
        }
        if is_registration_method(&m.method) {
            continue;
        }
        if !method_belongs_to_service(m, &svc_lower) {
            continue;
        }
        let class_lower = m.class.to_lowercase();
        if class_lower.contains(&svc_lower) || svc_lower.contains(&class_lower) {
            let method_lower = m.method.to_lowercase();
            if method_lower.contains("get")
                || method_lower.contains("post")
                || method_lower.contains("handle")
            {
                return (Some(format!("{}.{}", m.class, m.method)), Some(0.40));
            }
        }
    }

    (None, None)
}

fn is_handler_class(class: &str) -> bool {
    let lower = class.to_lowercase();
    lower.ends_with("controller")
        || lower.ends_with("api")
        || lower.ends_with("service")
        || lower.ends_with("handler")
        || lower.ends_with("endpoint")
        || lower.contains("eventhandler")
}

/// Check if a method node belongs to the given SDG service name
fn method_belongs_to_service(m: &crate::code::types::MethodNode, svc_lower: &str) -> bool {
    m.service.as_ref().is_some_and(|s| {
        let s_lower = s.to_lowercase();
        s_lower.contains(svc_lower) || svc_lower.contains(&s_lower)
    })
}

/// Keep whichever candidate has the more specific (longer) route pattern.
fn pick_more_specific<'a>(
    current: Option<&'a crate::code::route::RouteEntry>,
    candidate: &'a crate::code::route::RouteEntry,
) -> &'a crate::code::route::RouteEntry {
    match current {
        Some(c)
            if c.route_pattern.matches('/').count()
                >= candidate.route_pattern.matches('/').count() =>
        {
            c
        }
        _ => candidate,
    }
}

/// Does a route entry's handler class live in the given service?
///
/// Service names in traces (`Agency.API`) rarely equal project directory names
/// (`Agency.API` here, but `Agency.Backend` or `agency` elsewhere), so the
/// comparison is substring-based in both directions. When the handler class is
/// absent from the code graph (an unmapped or external dependency), fall back
/// to a name-only check so genuinely external calls are not credited to a
/// local class.
fn route_class_belongs_to_service(
    entry: &crate::code::route::RouteEntry,
    service: &str,
    code_graph: &CodeGraph,
) -> bool {
    let svc_lower = service.to_lowercase();
    if svc_lower.is_empty() {
        return true;
    }
    match code_graph.find_method(&entry.class, &entry.method) {
        Some(mn) => method_belongs_to_service(mn, &svc_lower),
        None => {
            let cls = entry.class.to_lowercase();
            !cls.is_empty()
                && (cls.contains(&svc_lower)
                    || svc_lower.contains(&cls)
                    || svc_lower
                        .split('.')
                        .any(|t| !t.is_empty() && cls.contains(t)))
        }
    }
}

fn is_registration_method(method: &str) -> bool {
    let lower = method.to_lowercase();
    lower.starts_with("map") || lower.starts_with("register") || lower.starts_with("configure")
}

fn try_grpc_method(endpoint: &str, code_graph: &CodeGraph) -> Option<String> {
    // gRPC endpoint format: "POST /package.Service/Method" or just "/Service/Method"
    let path = endpoint.split(' ').nth(1).unwrap_or("");
    if path.is_empty() {
        return None;
    }
    // Extract the last two parts: Service/Method
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let grpc_method = parts[parts.len() - 1];
    let grpc_service = parts[parts.len() - 2];

    // Try exact match: class name contains the gRPC service name, method contains the gRPC method
    for m in &code_graph.methods {
        if m.class == m.method {
            continue;
        }
        let class_lower = m.class.to_lowercase();
        let svc_lower = grpc_service.to_lowercase();
        if class_lower.contains(&svc_lower) || svc_lower.contains(&class_lower) {
            let method_lower = m.method.to_lowercase();
            let mtd_lower = grpc_method.to_lowercase();
            if method_lower.contains(&mtd_lower) || mtd_lower.contains(&method_lower) {
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
        // Find method-level mapping with confidence scores
        let ((src_method, src_conf), (tgt_method, tgt_conf)) = map_service_edge_to_method(
            &edge.source,
            &edge.target,
            &edge.source_endpoint,
            &edge.target_endpoint,
            code_graph,
            route_table,
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
                    &edge.source,
                    class,
                    method,
                    &mn.file,
                    mn.line_start,
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
                    &edge.target,
                    class,
                    method,
                    &mn.file,
                    mn.line_start,
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
                source_confidence: src_conf,
                target_service: edge.target.clone(),
                target_method: tgt_method.clone(),
                target_confidence: tgt_conf,
                target_file: None, // will be set from method node
                target_line: None,
                trace_id: Some(trace_id.clone()),
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
    let mut out: Vec<UnifiedEdge> = Vec::with_capacity(edges.len());

    for edge in edges {
        let key = (edge.source_id.clone(), edge.target_id.clone());
        let entry = merged.entry(key).or_insert(UnifiedEdge {
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            source_service: edge.source_service.clone(),
            source_method: edge.source_method.clone(),
            source_confidence: edge.source_confidence,
            target_service: edge.target_service.clone(),
            target_method: edge.target_method.clone(),
            target_confidence: edge.target_confidence,
            target_file: edge.target_file.clone(),
            target_line: edge.target_line,
            // Set below once every contributing edge has been folded in.
            trace_id: None,
            error: edge.error,
            latency_ms: edge.latency_ms,
            call_count: 0,
        });
        entry.call_count += edge.call_count;
        entry.error = entry.error || edge.error;
        entry.latency_ms = (entry.latency_ms * (entry.call_count - 1) as f64 + edge.latency_ms)
            / entry.call_count as f64;
    }

    // Keep one representative trace id per merged edge. Merging previously
    // dropped trace ids entirely, which made `tracepath --trace-id` unable to
    // match anything. Several traces can fold into a single edge, and the edge
    // carries one `trace_id`, so keep the first contributor: it is
    // deterministic for a given input order and enough to satisfy an exact
    // `--trace-id` filter. call_count still reports the true total.
    let mut seen: HashMap<(String, String), String> = HashMap::new();
    for edge in edges {
        let key = (edge.source_id.clone(), edge.target_id.clone());
        if let Some(tid) = edge.trace_id.as_ref() {
            seen.entry(key).or_insert_with(|| tid.clone());
        }
    }
    for (key, mut edge) in merged {
        edge.trace_id = seen.get(&key).cloned();
        out.push(edge);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::route::{RouteEntry, RouteTable};
    use crate::code::types::{CodeGraph, MethodNode};
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
        sdg.add_edge(
            "CatalogAPI",
            "OrderingAPI",
            "GET /api/items/{id}",
            "POST /validate",
            false,
            150.0,
            "trace1",
        );
        sdg
    }

    #[test]
    fn test_map_service_edge_route_match() {
        let cg = make_code_graph();
        let rt = make_route_table();
        let ((src, src_conf), (tgt, tgt_conf)) = map_service_edge_to_method(
            "CatalogAPI",
            "OrderingAPI",
            "GET /api/items/{id}",
            "POST /validate",
            &cg,
            &rt,
        );
        assert_eq!(src, Some("CatalogApi.GetItemById".into()));
        assert_eq!(src_conf, Some(0.95));
        assert_eq!(tgt, Some("OrderValidator.Validate".into()));
        assert_eq!(tgt_conf, Some(0.95));
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

    /// A merged route table must not hand one service another service's
    /// handler. Both projects register a `/api/{id}/member` style pattern, and
    /// the unscoped match previously returned the wrong class at full 0.95
    /// confidence.
    #[test]
    fn test_route_match_is_scoped_to_service() {
        let mut cg = CodeGraph::new();
        cg.add_method(MethodNode {
            class: "AgencyMemberEndpoint".into(),
            method: "Handler".into(),
            signature: "Task<IResult> Handler(Guid id)".into(),
            file: "Endpoints/Members.cs".into(),
            line_start: 10,
            line_end: 30,
            service: Some("Agency.API".into()),
        });
        cg.add_method(MethodNode {
            class: "BillingMemberEndpoint".into(),
            method: "Handler".into(),
            signature: "Task<IResult> Handler(Guid id)".into(),
            file: "Endpoints/Members.cs".into(),
            line_start: 40,
            line_end: 60,
            service: Some("Billing.API".into()),
        });

        let mut rt = RouteTable::new();
        rt.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/agency/{agencyId}/member".into(),
            class: "AgencyMemberEndpoint".into(),
            method: "Handler".into(),
        });
        rt.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/billing/{accountId}/member".into(),
            class: "BillingMemberEndpoint".into(),
            method: "Handler".into(),
        });

        let (agency, conf) = map_endpoint("Agency.API", "GET /api/agency/9f1c/member", &cg, &rt);
        assert_eq!(agency, Some("AgencyMemberEndpoint.Handler".into()));
        assert_eq!(conf, Some(0.95));

        let (billing, conf) = map_endpoint("Billing.API", "GET /api/billing/ab12/member", &cg, &rt);
        assert_eq!(billing, Some("BillingMemberEndpoint.Handler".into()));
        assert_eq!(conf, Some(0.95));
    }

    /// When no in-service route matches but another service's pattern does, the
    /// match is still reported, at reduced confidence.
    #[test]
    fn test_cross_service_route_match_is_lower_confidence() {
        let mut cg = CodeGraph::new();
        cg.add_method(MethodNode {
            class: "AgencyMemberEndpoint".into(),
            method: "Handler".into(),
            signature: "Task<IResult> Handler(Guid id)".into(),
            file: "Endpoints/Members.cs".into(),
            line_start: 10,
            line_end: 30,
            service: Some("Agency.API".into()),
        });
        let mut rt = RouteTable::new();
        rt.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/agency/{agencyId}/member".into(),
            class: "AgencyMemberEndpoint".into(),
            method: "Handler".into(),
        });

        // Frontend has no code, so the only candidate is another service's.
        let (method, conf) = map_endpoint("Frontend", "GET /api/agency/9f1c/member", &cg, &rt);
        assert_eq!(method, Some("AgencyMemberEndpoint.Handler".into()));
        assert_eq!(conf, Some(0.70), "cross-service match must not claim 0.95");
    }

    #[test]
    fn test_map_endpoint_route_table_first() {
        let cg = make_code_graph();
        let rt = make_route_table();
        let (method, confidence) = map_endpoint("CatalogAPI", "GET /api/items/123", &cg, &rt);
        assert_eq!(method, Some("CatalogApi.GetItemById".into()));
        assert_eq!(confidence, Some(0.95));
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
        let (method, confidence) = map_endpoint("API", "GET /health", &cg, &rt);
        // "GET /health" vs method "HealthCheck": the keyword fallback matches on
        // the last path segment, so this resolves. Assert the real outcome
        // instead of `is_some() || is_none()`, which passes either way.
        assert_eq!(method, Some("HealthController.HealthCheck".into()));
        assert_eq!(confidence, Some(0.60));
    }

    #[test]
    fn test_merge_duplicate_edges() {
        let edges = vec![
            UnifiedEdge {
                source_id: "svc:A".into(),
                target_id: "svc:B".into(),
                source_service: "A".into(),
                source_method: None,
                source_confidence: None,
                target_service: "B".into(),
                target_method: None,
                target_confidence: None,
                target_file: None,
                target_line: None,
                trace_id: None,
                error: false,
                latency_ms: 100.0,
                call_count: 1,
            },
            UnifiedEdge {
                source_id: "svc:A".into(),
                target_id: "svc:B".into(),
                source_service: "A".into(),
                source_method: None,
                source_confidence: None,
                target_service: "B".into(),
                target_method: None,
                target_confidence: None,
                target_file: None,
                target_line: None,
                trace_id: None,
                error: true,
                latency_ms: 200.0,
                call_count: 1,
            },
        ];
        let merged = merge_duplicate_edges(&edges);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].call_count, 2);
        assert!(merged[0].error);
    }
}
