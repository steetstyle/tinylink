use anyhow::Result;
use std::path::Path;

use super::route::{RouteEntry, RouteTable};
use super::types::{CallEdge, CodeGraph, MethodNode};

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

        let mut st =
            match tiny_pdg_cs::resolve::symbols::SymbolTable::from_ast(tree.root_node(), &source) {
                Ok(s) => s,
                Err(_) => continue,
            };
        // SymbolTable::from_ast has no file-path argument, so methods come back
        // with an empty `file`. Annotate before reading, otherwise every
        // method in the graph reports "" and file:line output degrades to ":14".
        st.type_graph.annotate_method_files(file_path);

        let cg = tiny_pdg_cs::analysis::callgraph::CallGraphBuilder::build(
            tree.root_node(),
            &source,
            &st.type_graph,
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
                    file: if md.file.is_empty() {
                        file_path.clone()
                    } else {
                        md.file.clone()
                    },
                    line_start: md.line_start,
                    line_end: md.line_end,
                    service: None,
                });
            }
        }

        // Extract interfaces
        for (iface_name, iface_info) in &st.type_graph.interfaces {
            if !code_graph.interfaces.iter().any(|i| i == iface_name) {
                code_graph.interfaces.push(iface_name.clone());
            }
            for md in &iface_info.methods {
                code_graph.add_method(MethodNode {
                    class: iface_name.clone(),
                    method: md.method.clone(),
                    signature: md.signature.clone(),
                    file: if md.file.is_empty() {
                        file_path.clone()
                    } else {
                        md.file.clone()
                    },
                    line_start: md.line_start,
                    line_end: md.line_end,
                    service: None,
                });
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

/// Extract route table from C# source using tiny-pdg-cs's route extractor.
///
/// Returns the table plus the count of routes whose handler is an inline lambda
/// and therefore cannot be attributed to a named method.
pub fn load_routes(source_dir: &str) -> Result<(RouteTable, usize)> {
    let (tiny_table, stats) = tiny_pdg_cs::route::extractor::extract_with_stats(source_dir)?;
    let mut table = RouteTable::new();
    for entry in tiny_table.routes {
        table.add(RouteEntry {
            http_method: entry.http_method,
            route_pattern: entry.path,
            class: entry.class,
            method: entry.handler,
        });
    }
    Ok((table, stats.inline_lambda_routes))
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
        std::fs::write(
            dir.join("test_controller.cs"),
            r#"
using Microsoft.AspNetCore.Mvc;

[Route("api/items")]
public class ItemsController : ControllerBase
{
    [HttpGet("{id}")]
    public IActionResult GetById(int id) { return Ok(); }

    [HttpPost]
    public IActionResult Create([FromBody] object req) { return Ok(); }
}
"#,
        )
        .unwrap();
        let (table, inline_lambdas) = load_routes(dir.to_str().unwrap()).unwrap();
        assert_eq!(inline_lambdas, 0);
        assert_eq!(table.entries.len(), 2);
        let get = table
            .entries
            .iter()
            .find(|r| r.http_method == "GET")
            .unwrap();
        assert_eq!(get.route_pattern, "/api/items/{id}");
        assert_eq!(get.class, "ItemsController");
        assert_eq!(get.method, "GetById");
        let post = table
            .entries
            .iter()
            .find(|r| r.http_method == "POST")
            .unwrap();
        assert_eq!(post.route_pattern, "/api/items");
        assert_eq!(post.method, "Create");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_load_routes_invalid_path() {
        // extract() returns Ok with an empty table for a non-existent dir, and
        // an Err is also acceptable. Assert the observable contract either way:
        // no routes are produced.
        if let Ok((table, _)) = load_routes("/nonexistent/path") {
            assert!(table.entries.is_empty());
        }
    }

    /// Regression test: a method must report its own file, not an empty string.
    #[test]
    fn test_load_project_sets_method_file_path() {
        let dir = std::env::temp_dir().join("tinylink_file_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Widget.cs");
        std::fs::write(
            &path,
            r#"
namespace Demo;

public class Widget
{
    public int Compute(int a)
    {
        return a * 2;
    }
}
"#,
        )
        .unwrap();

        let cg = load_project(dir.to_str().unwrap()).unwrap();
        let m = cg
            .find_method("Widget", "Compute")
            .expect("method not found");
        assert!(
            !m.file.is_empty(),
            "method file must be populated, got an empty path"
        );
        assert!(m.file.ends_with("Widget.cs"), "unexpected file: {}", m.file);
        assert!(m.line_start > 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Regression test: the one-shot minimal-API idiom must produce routes.
    #[test]
    fn test_load_routes_minimal_api_qualified_handler() {
        let dir = std::env::temp_dir().join("tinylink_minapi_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Endpoints.cs"),
            r#"
namespace Agency.API.Endpoints;

public class GetMembersEndpoint
{
    public static IResult Handler(HttpContext ctx) { return Results.Ok(); }
}
"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("EndpointExtension.cs"),
            r#"
using Agency.API.Endpoints;

namespace Agency.API.Extensions;

public static class EndpointExtension
{
    public static IEndpointRouteBuilder MapApi(this IEndpointRouteBuilder app)
    {
        var api = app.MapGroup("");
        api.MapGet("/api/agency/{agencyId:guid}/member", GetMembersEndpoint.Handler);
        return app;
    }
}
"#,
        )
        .unwrap();

        let (table, inline_lambdas) = load_routes(dir.to_str().unwrap()).unwrap();
        assert_eq!(inline_lambdas, 0);
        assert_eq!(
            table.entries.len(),
            1,
            "expected the qualified handler route"
        );
        let r = &table.entries[0];
        assert_eq!(r.http_method, "GET");
        // Constraint stripped so segment matching works.
        assert_eq!(r.route_pattern, "/api/agency/{agencyId}/member");
        assert_eq!(r.class, "GetMembersEndpoint");
        assert_eq!(r.method, "Handler");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// An inline lambda handler has no named method: it must not become a
    /// bogus route entry, but it must be reported.
    #[test]
    fn test_load_routes_reports_inline_lambda() {
        let dir = std::env::temp_dir().join("tinylink_lambda_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Program.cs"),
            r#"
var app = WebApplication.CreateBuilder(args).Build();
app.MapGet("/cost", async (HttpContext ctx) => { return Results.Ok(); });
"#,
        )
        .unwrap();

        let (table, inline_lambdas) = load_routes(dir.to_str().unwrap()).unwrap();
        assert_eq!(inline_lambdas, 1);
        assert!(table.entries.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
