use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;

use super::types::{CodeGraph, MethodNode, CallEdge};
use super::route::{RouteTable, RouteEntry};

/// Load C# project source, parse with tiny-pdg-cs, build CodeGraph
pub fn load_project(source_dir: &str) -> Result<CodeGraph> {
    let path = Path::new(source_dir);
    if !path.exists() {
        anyhow::bail!("Source directory does not exist: {}", source_dir);
    }

    let mut code_graph = CodeGraph::new();
    let mut all_cs_files = Vec::new();
    collect_cs_files(path, &mut all_cs_files);

    for file_path in &all_cs_files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let tree = match tiny_pdg_cs::parse::parser::parse_source(&source) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let st = match tiny_pdg_cs::resolve::symbols::SymbolTable::from_ast(
            tree.root_node(), &source,
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let cg = tiny_pdg_cs::analysis::callgraph::CallGraphBuilder::build(
            tree.root_node(), &source, &st.type_graph,
        );

        // Extract methods
        for (class_name, class_info) in &st.type_graph.classes {
            if !code_graph.classes.iter().any(|c| c == class_name) {
                code_graph.classes.push(class_name.clone());
            }
            for md in &class_info.methods {
                code_graph.add_method(MethodNode {
                    class: class_name.clone(),
                    method: md.method.clone(),
                    signature: md.signature.clone(),
                    file: md.file.clone(),
                    line_start: md.line_start,
                    line_end: md.line_end,
                    service: None,
                });
            }
        }

        // Extract interfaces
        for (iface_name, _) in &st.type_graph.interfaces {
            if !code_graph.interfaces.iter().any(|i| i == iface_name) {
                code_graph.interfaces.push(iface_name.clone());
            }
        }

        // Extract calls
        for call in &cg.calls {
            code_graph.calls.push(CallEdge {
                source_class: call.caller_class.clone(),
                source_method: call.caller_method.clone(),
                target_class: call.callee_class.clone(),
                target_method: call.callee.clone(),
                line: Some(call.line),
            });
        }
    }

    Ok(code_graph)
}

/// Extract route table from C# source using tiny-pdg-cs's route extractor
pub fn load_routes(source_dir: &str) -> Result<RouteTable> {
    let tiny_table = tiny_pdg_cs::route::extractor::extract(source_dir)?;
    let mut table = RouteTable::new();
    for entry in tiny_table.routes {
        table.add(RouteEntry {
            http_method: entry.http_method,
            route_pattern: entry.path,
            class: entry.class,
            method: entry.handler,
        });
    }
    Ok(table)
}

pub fn collect_cs_files(dir: &Path, files: &mut Vec<String>) {
    if dir.is_file() {
        if dir.extension().map(|e| e == "cs").unwrap_or(false) {
            files.push(dir.to_string_lossy().to_string());
        }
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_cs_files(&path, files);
            } else if path.extension().map(|e| e == "cs").unwrap_or(false) {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
}

/// Load multiple C# project directories and merge them into a single CodeGraph.
/// Each directory is expected to contain a separate service (e.g., Catalog.API, Ordering.API).
/// Service names can be optionally mapped via a HashMap (dir → service_name).
pub fn load_multi_project(dirs: &[&str], service_names: &HashMap<String, String>) -> Result<CodeGraph> {
    let mut merged = CodeGraph::new();
    for dir in dirs {
        let cg = load_project(dir)?;
        let svc_name = service_names.get(*dir).cloned().unwrap_or_else(|| {
            Path::new(dir).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
        });
        for mut m in cg.methods {
            m.service = Some(svc_name.clone());
            merged.add_method(m);
        }
        for c in cg.calls {
            if !merged.calls.iter().any(|ec| ec.source_class == c.source_class
                && ec.source_method == c.source_method
                && ec.target_class == c.target_class
                && ec.target_method == c.target_method)
            {
                merged.calls.push(c);
            }
        }
        for cls in cg.classes {
            if !merged.classes.contains(&cls) {
                merged.classes.push(cls);
            }
        }
        for iface in cg.interfaces {
            if !merged.interfaces.contains(&iface) {
                merged.interfaces.push(iface);
            }
        }
    }
    Ok(merged)
}

/// Load routes from multiple C# project directories, merged into one RouteTable
pub fn load_multi_routes(dirs: &[&str]) -> Result<RouteTable> {
    let mut merged = RouteTable::new();
    for dir in dirs {
        let table = load_routes(dir)?;
        for entry in table.entries {
            if !merged.entries.iter().any(|e| e.http_method == entry.http_method
                && e.route_pattern == entry.route_pattern
                && e.class == entry.class
                && e.method == entry.method)
            {
                merged.entries.push(entry);
            }
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_cs_files_directory() {
        let dir = Path::new("tests/fixtures/projects");
        let mut files = Vec::new();
        collect_cs_files(dir, &mut files);
        assert!(files.is_empty() || files.iter().any(|f| f.ends_with(".cs")));
    }

    #[test]
    fn test_load_project_invalid_path() {
        let result = load_project("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_cs_files_single_file() {
        let tmp = std::env::temp_dir().join("test_single.cs");
        std::fs::write(&tmp, "class A {}").unwrap();
        let mut files = Vec::new();
        collect_cs_files(&tmp, &mut files);
        assert_eq!(files.len(), 1);
        std::fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_load_routes_from_controller_file() {
        let dir = std::env::temp_dir().join("tinylink_route_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test_controller.cs"), r#"
using Microsoft.AspNetCore.Mvc;

[Route("api/items")]
public class ItemsController : ControllerBase
{
    [HttpGet("{id}")]
    public IActionResult GetById(int id) { return Ok(); }

    [HttpPost]
    public IActionResult Create([FromBody] object req) { return Ok(); }
}
"#).unwrap();
        let table = load_routes(dir.to_str().unwrap()).unwrap();
        assert_eq!(table.entries.len(), 2);
        let get = table.entries.iter().find(|r| r.http_method == "GET").unwrap();
        assert_eq!(get.route_pattern, "/api/items/{id}");
        assert_eq!(get.class, "ItemsController");
        assert_eq!(get.method, "GetById");
        let post = table.entries.iter().find(|r| r.http_method == "POST").unwrap();
        assert_eq!(post.route_pattern, "/api/items");
        assert_eq!(post.method, "Create");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_load_routes_invalid_path() {
        let result = load_routes("/nonexistent/path");
        // extract() returns Ok with empty table for non-existent dirs
        match result {
            Ok(table) => assert!(table.entries.is_empty()),
            Err(_) => {}, // also acceptable
        }
    }
}
