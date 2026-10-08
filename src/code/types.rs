use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodNode {
    pub class: String,
    pub method: String,
    pub signature: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub source_class: String,
    pub source_method: String,
    pub target_class: String,
    pub target_method: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    pub methods: Vec<MethodNode>,
    pub calls: Vec<CallEdge>,
    pub classes: Vec<String>,
    pub interfaces: Vec<String>,
}

impl Default for CodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGraph {
    pub fn new() -> Self {
        CodeGraph {
            methods: Vec::new(),
            calls: Vec::new(),
            classes: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    pub fn find_method(&self, class: &str, method: &str) -> Option<&MethodNode> {
        self.methods
            .iter()
            .find(|m| m.class == class && m.method == method)
    }

    pub fn add_method(&mut self, node: MethodNode) {
        if !self
            .methods
            .iter()
            .any(|m| m.class == node.class && m.method == node.method)
        {
            self.methods.push(node);
        }
    }

    pub fn to_dot(&self) -> String {
        let mut dot = String::from(
            "digraph CodeGraph {\n  rankdir=LR;\n  node [shape=box style=rounded];\n\n",
        );
        for m in &self.methods {
            let label = format!("{}.{}", m.class, m.method);
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\\n{}.{}:{}\"];\n",
                label, label, m.file, m.line_start, m.line_end
            ));
        }
        dot.push('\n');
        for c in &self.calls {
            let src = format!("{}.{}", c.source_class, c.source_method);
            let tgt = format!("{}.{}", c.target_class, c.target_method);
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", src, tgt));
        }
        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code_graph() {
        let g = CodeGraph::new();
        assert!(g.methods.is_empty());
        assert!(g.calls.is_empty());
    }

    #[test]
    fn test_add_method_deduplicates() {
        let mut g = CodeGraph::new();
        g.add_method(MethodNode {
            class: "A".into(),
            method: "foo".into(),
            signature: "void foo()".into(),
            file: "a.cs".into(),
            line_start: 1,
            line_end: 5,
            service: None,
        });
        g.add_method(MethodNode {
            class: "A".into(),
            method: "foo".into(),
            signature: "void foo()".into(),
            file: "a.cs".into(),
            line_start: 1,
            line_end: 5,
            service: None,
        });
        assert_eq!(g.methods.len(), 1);
    }

    #[test]
    fn test_find_method() {
        let mut g = CodeGraph::new();
        g.add_method(MethodNode {
            class: "A".into(),
            method: "foo".into(),
            signature: "void foo()".into(),
            file: "a.cs".into(),
            line_start: 1,
            line_end: 5,
            service: None,
        });
        assert!(g.find_method("A", "foo").is_some());
        assert!(g.find_method("A", "bar").is_none());
    }

    #[test]
    fn test_to_dot_contains_methods() {
        let mut g = CodeGraph::new();
        g.add_method(MethodNode {
            class: "A".into(),
            method: "foo".into(),
            signature: "void foo()".into(),
            file: "a.cs".into(),
            line_start: 1,
            line_end: 5,
            service: None,
        });
        let dot = g.to_dot();
        assert!(dot.contains("A.foo"));
        assert!(dot.contains("digraph CodeGraph"));
    }
}
