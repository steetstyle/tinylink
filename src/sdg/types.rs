use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    pub name: String,
    pub endpoints: Vec<String>,
    pub error_count: usize,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEdge {
    pub source: String,
    pub target: String,
    pub source_endpoint: String,
    pub target_endpoint: String,
    pub call_count: usize,
    pub error_count: usize,
    pub avg_latency_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub trace_ids: Vec<String>,
    #[serde(skip)]
    pub latency_samples: Vec<f64>,
}

impl ServiceEdge {
    /// Recompute P50/P95/P99 from stored latency_samples.
    pub fn compute_percentiles(&mut self) {
        if self.latency_samples.is_empty() {
            self.latency_p50_ms = self.avg_latency_ms;
            self.latency_p95_ms = self.avg_latency_ms;
            self.latency_p99_ms = self.avg_latency_ms;
            return;
        }
        let mut sorted = self.latency_samples.clone();
        sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let idx50 = ((n as f64 * 0.50) as usize).min(n - 1);
        let idx95 = ((n as f64 * 0.95) as usize).min(n - 1);
        let idx99 = ((n as f64 * 0.99) as usize).min(n - 1);
        self.latency_p50_ms = sorted[idx50];
        self.latency_p95_ms = sorted[idx95];
        self.latency_p99_ms = sorted[idx99];
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGraph {
    pub nodes: HashMap<String, ServiceNode>,
    pub edges: Vec<ServiceEdge>,
    pub total_traces: usize,
    pub error_traces: usize,
}

impl Default for ServiceGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceGraph {
    pub fn new() -> Self {
        ServiceGraph {
            nodes: HashMap::new(),
            edges: Vec::new(),
            total_traces: 0,
            error_traces: 0,
        }
    }

    pub fn ensure_node(&mut self, name: &str) {
        self.nodes.entry(name.to_string()).or_insert(ServiceNode {
            name: name.to_string(),
            endpoints: Vec::new(),
            error_count: 0,
            avg_latency_ms: 0.0,
        });
    }

    pub fn add_endpoint(&mut self, service: &str, endpoint: &str) {
        if let Some(node) = self.nodes.get_mut(service) {
            if !node.endpoints.contains(&endpoint.to_string()) {
                node.endpoints.push(endpoint.to_string());
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_edge(
        &mut self,
        source: &str,
        target: &str,
        source_endpoint: &str,
        target_endpoint: &str,
        error: bool,
        latency_ms: f64,
        trace_id: &str,
    ) {
        self.ensure_node(source);
        self.ensure_node(target);
        self.add_endpoint(source, source_endpoint);
        self.add_endpoint(target, target_endpoint);

        if let Some(edge) = self.edges.iter_mut().find(|e| {
            e.source == source
                && e.target == target
                && e.source_endpoint == source_endpoint
                && e.target_endpoint == target_endpoint
        }) {
            edge.call_count += 1;
            if error {
                edge.error_count += 1;
            }
            edge.avg_latency_ms = (edge.avg_latency_ms * (edge.call_count - 1) as f64 + latency_ms)
                / edge.call_count as f64;
            edge.latency_samples.push(latency_ms);
            edge.compute_percentiles();
            if !edge.trace_ids.contains(&trace_id.to_string()) {
                edge.trace_ids.push(trace_id.to_string());
            }
        } else {
            let mut edge = ServiceEdge {
                source: source.to_string(),
                target: target.to_string(),
                source_endpoint: source_endpoint.to_string(),
                target_endpoint: target_endpoint.to_string(),
                call_count: 1,
                error_count: if error { 1 } else { 0 },
                avg_latency_ms: latency_ms,
                latency_p50_ms: latency_ms,
                latency_p95_ms: latency_ms,
                latency_p99_ms: latency_ms,
                trace_ids: vec![trace_id.to_string()],
                latency_samples: vec![latency_ms],
            };
            edge.compute_percentiles();
            self.edges.push(edge);
        }

        if error {
            if let Some(n) = self.nodes.get_mut(source) {
                n.error_count += 1;
            }
            if let Some(n) = self.nodes.get_mut(target) {
                n.error_count += 1;
            }
        }
    }

    pub fn merge(&mut self, other: ServiceGraph) {
        for (name, node) in other.nodes {
            let entry = self.nodes.entry(name).or_insert(ServiceNode {
                name: node.name.clone(),
                endpoints: Vec::new(),
                error_count: 0,
                avg_latency_ms: 0.0,
            });
            entry.error_count += node.error_count;
            entry.avg_latency_ms = (entry.avg_latency_ms + node.avg_latency_ms) / 2.0;
            for ep in node.endpoints {
                if !entry.endpoints.contains(&ep) {
                    entry.endpoints.push(ep);
                }
            }
        }
        for mut e in other.edges {
            e.compute_percentiles();
            self.edges.push(e);
        }
        self.total_traces += other.total_traces;
        self.error_traces += other.error_traces;
    }

    pub fn to_dot(&self) -> String {
        let mut dot =
            String::from("digraph SDG {\n  rankdir=LR;\n  node [shape=box style=rounded];\n\n");
        for (name, node) in &self.nodes {
            let color = if node.error_count > 0 {
                "color=red penwidth=2"
            } else {
                ""
            };
            let extra = if node.error_count > 0 {
                format!("\\nerrors: {}", node.error_count)
            } else {
                String::new()
            };
            dot.push_str(&format!(
                "  {} [label=\"{}{}\" {}];\n",
                name, name, extra, color
            ));
        }
        dot.push('\n');
        for edge in &self.edges {
            let color = if edge.error_count > 0 {
                "color=red penwidth=2"
            } else {
                "color=blue"
            };
            let err_info = if edge.error_count > 0 {
                format!(" ERRORS:{}", edge.error_count)
            } else {
                String::new()
            };
            dot.push_str(&format!(
                "  {} -> {} [{} label=\"{}\\ncalls:{}{} avg:{:.0}ms P50:{:.0} P95:{:.0} P99:{:.0}\"];\n",
                edge.source, edge.target, color, edge.target_endpoint,
                edge.call_count, err_info,
                edge.avg_latency_ms, edge.latency_p50_ms,
                edge.latency_p95_ms, edge.latency_p99_ms,
            ));
        }
        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let g = ServiceGraph::new();
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn test_add_edge_creates_nodes() {
        let mut g = ServiceGraph::new();
        g.add_edge(
            "CatalogAPI",
            "OrderingAPI",
            "GET /items",
            "POST /validate",
            false,
            150.0,
            "trace1",
        );
        assert_eq!(g.nodes.len(), 2);
        assert!(g.nodes.contains_key("CatalogAPI"));
        assert!(g.nodes.contains_key("OrderingAPI"));
    }

    #[test]
    fn test_add_edge_aggregates_counts() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", false, 100.0, "t1");
        g.add_edge("A", "B", "ep1", "ep2", true, 200.0, "t2");
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].call_count, 2);
        assert_eq!(g.edges[0].error_count, 1);
        assert!((g.edges[0].avg_latency_ms - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_add_edge_separate_endpoints() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "GET /a", "POST /b1", false, 100.0, "t1");
        g.add_edge("A", "B", "GET /a", "POST /b2", true, 200.0, "t2");
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn test_merge() {
        let mut g1 = ServiceGraph::new();
        g1.add_edge("A", "B", "ep1", "ep2", false, 100.0, "t1");
        let mut g2 = ServiceGraph::new();
        g2.add_edge("B", "C", "ep3", "ep4", true, 50.0, "t2");
        g1.merge(g2);
        assert_eq!(g1.nodes.len(), 3);
        assert_eq!(g1.edges.len(), 2);
    }

    #[test]
    fn test_error_counts_propagate_to_nodes() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", true, 100.0, "t1");
        assert_eq!(g.nodes.get("A").unwrap().error_count, 1);
        assert_eq!(g.nodes.get("B").unwrap().error_count, 1);
    }

    #[test]
    fn test_to_dot_contains_nodes() {
        let mut g = ServiceGraph::new();
        g.add_edge(
            "CatalogAPI",
            "OrderingAPI",
            "GET /items",
            "POST /validate",
            false,
            150.0,
            "t1",
        );
        let dot = g.to_dot();
        assert!(dot.contains("CatalogAPI"));
        assert!(dot.contains("OrderingAPI"));
        assert!(dot.contains("digraph SDG"));
    }

    #[test]
    fn test_to_dot_error_coloring() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", true, 100.0, "t1");
        let dot = g.to_dot();
        assert!(dot.contains("color=red"));
    }

    #[test]
    fn test_to_dot_no_error_default_color() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", false, 100.0, "t1");
        let dot = g.to_dot();
        assert!(!dot.contains("color=red"));
    }

    #[test]
    fn test_latency_percentiles() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", false, 10.0, "t1");
        g.add_edge("A", "B", "ep1", "ep2", false, 20.0, "t2");
        g.add_edge("A", "B", "ep1", "ep2", false, 30.0, "t3");
        g.add_edge("A", "B", "ep1", "ep2", false, 100.0, "t4");
        let e = &g.edges[0];
        assert!((e.latency_p50_ms - 30.0).abs() < 1.0);
        assert!((e.latency_p95_ms - 100.0).abs() < 1.0);
        assert!((e.latency_p99_ms - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_dot_shows_latency_percentiles() {
        let mut g = ServiceGraph::new();
        g.add_edge("A", "B", "ep1", "ep2", false, 150.0, "t1");
        let dot = g.to_dot();
        assert!(dot.contains("P50:"));
        assert!(dot.contains("P95:"));
        assert!(dot.contains("P99:"));
    }
}
