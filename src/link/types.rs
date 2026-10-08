use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedNodeKind {
    Service,
    Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNode {
    pub id: String,
    pub kind: UnifiedNodeKind,
    pub service: Option<String>,
    pub class: Option<String>,
    pub method: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEdge {
    pub source_id: String,
    pub target_id: String,
    pub source_service: String,
    pub source_method: Option<String>,
    pub source_confidence: Option<f64>,
    pub target_service: String,
    pub target_method: Option<String>,
    pub target_confidence: Option<f64>,
    pub target_file: Option<String>,
    pub target_line: Option<usize>,
    pub trace_id: Option<String>,
    pub error: bool,
    pub latency_ms: f64,
    pub call_count: usize,
}

/// Graph combining service-level and code-level dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedGraph {
    pub nodes: HashMap<String, UnifiedNode>,
    pub edges: Vec<UnifiedEdge>,
    pub total_traces: usize,
    pub error_traces: usize,
    pub services: Vec<String>,
}

impl Default for UnifiedGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedGraph {
    pub fn new() -> Self {
        UnifiedGraph {
            nodes: HashMap::new(),
            edges: Vec::new(),
            total_traces: 0,
            error_traces: 0,
            services: Vec::new(),
        }
    }

    pub fn add_service_node(&mut self, name: &str, error_count: usize) {
        let id = format!("svc:{}", name);
        self.nodes.entry(id.clone()).or_insert(UnifiedNode {
            id,
            kind: UnifiedNodeKind::Service,
            service: Some(name.to_string()),
            class: None,
            method: None,
            file: None,
            line: None,
            error_count,
        });
    }

    pub fn add_method_node(
        &mut self,
        service: &str,
        class: &str,
        method: &str,
        file: &str,
        line: usize,
        error_count: usize,
    ) -> String {
        let id = format!("method:{}:{}.{}", service, class, method);
        self.nodes.entry(id.clone()).or_insert(UnifiedNode {
            id: id.clone(),
            kind: UnifiedNodeKind::Method,
            service: Some(service.to_string()),
            class: Some(class.to_string()),
            method: Some(method.to_string()),
            file: Some(file.to_string()),
            line: Some(line),
            error_count,
        });
        id
    }

    pub fn add_edge(&mut self, edge: UnifiedEdge) {
        self.edges.push(edge);
    }

    pub fn to_dot(&self) -> String {
        let mut dot = String::from(
            "digraph UnifiedGraph {\n  rankdir=LR;\n  node [shape=box style=rounded];\n\n",
        );

        for node in self.nodes.values() {
            let label = match &node.kind {
                UnifiedNodeKind::Service => {
                    let svc = node.service.as_deref().unwrap_or("?");
                    if node.error_count > 0 {
                        format!(
                            "{} [color=red label=\"{}\\nerrors: {}\"]",
                            node.id, svc, node.error_count
                        )
                    } else {
                        format!("{} [label=\"{}\"]", node.id, svc)
                    }
                }
                UnifiedNodeKind::Method => {
                    let cls = node.class.as_deref().unwrap_or("?");
                    let m = node.method.as_deref().unwrap_or("?");
                    let fl = node.file.as_deref().unwrap_or("?");
                    let ln = node.line.unwrap_or(0);
                    format!(
                        "{} [shape=note label=\"{}.{}\\n{}:{}\"]",
                        node.id, cls, m, fl, ln
                    )
                }
            };
            dot.push_str(&format!("  {};\n", label));
        }
        dot.push('\n');

        for edge in &self.edges {
            let color = if edge.error {
                "color=red"
            } else {
                "color=blue"
            };
            let src_conf = edge
                .source_confidence
                .map(|c| format!(" c={:.2}", c))
                .unwrap_or_default();
            let tgt_conf = edge
                .target_confidence
                .map(|c| format!(" c={:.2}", c))
                .unwrap_or_default();
            let label = format!(
                "calls: {} errors: {} {:.0}ms | src{} tgt{}",
                edge.call_count,
                if edge.error { 1 } else { 0 },
                edge.latency_ms,
                src_conf,
                tgt_conf,
            );
            // Only the file portion of "path:line" when a path is known: an
            // empty file string previously produced a bare ":14".
            if let Some(file) = edge.target_file.as_deref().filter(|f| !f.is_empty()) {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\" [{} label=\"{}\\n{}:{}\"];\n",
                    edge.source_id,
                    edge.target_id,
                    color,
                    label,
                    file,
                    edge.target_line.unwrap_or(0),
                ));
            } else {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\" [{} label=\"{}\"];\n",
                    edge.source_id, edge.target_id, color, label,
                ));
            }
        }

        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_unified_graph() {
        let g = UnifiedGraph::new();
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn test_add_service_node() {
        let mut g = UnifiedGraph::new();
        g.add_service_node("CatalogAPI", 0);
        g.add_service_node("OrderingAPI", 2);
        assert_eq!(g.nodes.len(), 2);
        let key = "svc:CatalogAPI";
        assert!(g.nodes.contains_key(key));
        assert!(g.nodes.contains_key("svc:OrderingAPI"));
        assert_eq!(g.nodes.get("svc:OrderingAPI").unwrap().error_count, 2);
    }

    #[test]
    fn test_add_method_node_deduplicates() {
        let mut g = UnifiedGraph::new();
        let id1 = g.add_method_node("Svc", "A", "foo", "a.cs", 10, 0);
        let id2 = g.add_method_node("Svc", "A", "foo", "a.cs", 10, 0);
        assert_eq!(id1, id2);
        assert_eq!(g.nodes.len(), 1);
    }

    #[test]
    fn test_to_dot_contains_nodes() {
        let mut g = UnifiedGraph::new();
        g.add_service_node("API", 1);
        g.add_method_node("API", "Controller", "Get", "c.cs", 5, 0);
        let dot = g.to_dot();
        assert!(dot.contains("svc:API"));
        assert!(dot.contains("method:API:Controller.Get"));
    }

    #[test]
    fn test_to_dot_method_edge() {
        let mut g = UnifiedGraph::new();
        g.add_method_node("A", "X", "foo", "x.cs", 1, 0);
        g.add_method_node("B", "Y", "bar", "y.cs", 2, 0);
        g.add_edge(UnifiedEdge {
            source_id: "method:A:X.foo".into(),
            target_id: "method:B:Y.bar".into(),
            source_service: "A".into(),
            source_method: Some("foo".into()),
            source_confidence: Some(0.95),
            target_service: "B".into(),
            target_method: Some("bar".into()),
            target_confidence: Some(0.85),
            target_file: Some("y.cs".into()),
            target_line: Some(2),
            trace_id: None,
            error: false,
            latency_ms: 150.0,
            call_count: 5,
        });
        let dot = g.to_dot();
        assert!(dot.contains("method:A:X.foo"));
        assert!(dot.contains("method:B:Y.bar"));
    }
}
