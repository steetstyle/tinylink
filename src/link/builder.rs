use anyhow::{Context, Result};

use crate::code::loader;
use crate::code::route::RouteTable;
use crate::code::types::CodeGraph;
use crate::link::mapper;
use crate::link::types::{UnifiedEdge, UnifiedGraph};
use crate::sdg::otel;

/// Build a UnifiedGraph from an OTLP trace file and C# source directory.
/// Supports multiple comma-separated source directories (code projects).
pub fn build_unified(trace_path: &str, sources: &str) -> Result<UnifiedGraph> {
    let sdg = otel::parse_trace_file(trace_path)
        .with_context(|| format!("Failed to parse trace: {}", trace_path))?;

    let (code_graph, route_table) = load_merged_projects(sources)?;
    let unified = mapper::build_unified_from_sdg(&sdg, &code_graph, &route_table);
    Ok(unified)
}

/// Build a UnifiedGraph from pre-loaded components (used by explore).
pub fn build_unified_from_components(
    sdg: &crate::sdg::types::ServiceGraph,
    code_graph: &CodeGraph,
    route_table: &RouteTable,
) -> UnifiedGraph {
    mapper::build_unified_from_sdg(sdg, code_graph, route_table)
}

/// Build a trace path: walk the UnifiedGraph to show span-by-span flow.
/// Supports multiple comma-separated source directories.
pub fn build_trace_path(
    trace_path: &str,
    sources: &str,
    filter_trace_id: Option<&str>,
) -> Result<Vec<TraceStep>> {
    let unified = build_unified(trace_path, sources)?;
    let mut steps = Vec::new();

    for edge in &unified.edges {
        if let Some(tid) = filter_trace_id {
            if edge.trace_id.as_deref() != Some(tid) {
                continue;
            }
        }

        steps.push(TraceStep {
            source_service: edge.source_service.clone(),
            source_method: edge.source_method.clone(),
            target_service: edge.target_service.clone(),
            target_method: edge.target_method.clone(),
            target_file: edge.target_file.clone(),
            target_line: edge.target_line,
            error: edge.error,
            latency_ms: edge.latency_ms,
            trace_id: edge.trace_id.clone(),
        });
    }

    Ok(steps)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceStep {
    pub source_service: String,
    pub source_method: Option<String>,
    pub target_service: String,
    pub target_method: Option<String>,
    pub target_file: Option<String>,
    pub target_line: Option<usize>,
    pub error: bool,
    pub latency_ms: f64,
    pub trace_id: Option<String>,
}

/// Query error paths in the unified graph.
/// Supports multiple comma-separated source directories.
pub fn query_errors(
    trace_path: &str,
    sources: &str,
    service_filter: Option<&str>,
) -> Result<Vec<UnifiedEdge>> {
    let unified = build_unified(trace_path, sources)?;

    let errors: Vec<_> = unified
        .edges
        .into_iter()
        .filter(|e| e.error)
        .filter(|e| {
            if let Some(svc) = service_filter {
                e.source_service == svc || e.target_service == svc
            } else {
                true
            }
        })
        .collect();

    Ok(errors)
}

/// Per-project load result. `error` is set instead of aborting so callers that
/// can degrade gracefully (explore) can report which project failed, while
/// strict callers bail on it.
#[derive(Debug)]
pub struct ProjectLoad {
    pub dir: String,
    pub service: String,
    pub code: Option<CodeGraph>,
    pub routes: Option<RouteTable>,
    pub error: Option<String>,
    /// Routes that exist in source but have no named handler (inline lambda).
    pub inline_lambda_routes: usize,
}

/// Service name for a project directory: its final path component.
pub fn service_name_for_dir(dir: &str) -> String {
    std::path::Path::new(dir)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Load one project directory, tagging its methods with the service name.
pub fn load_project(dir: &str) -> ProjectLoad {
    let service = service_name_for_dir(dir);

    let mut code = match loader::load_project(dir) {
        Ok(mut cg) => {
            for m in &mut cg.methods {
                m.service = Some(service.clone());
            }
            Some(cg)
        }
        Err(e) => {
            return ProjectLoad {
                dir: dir.to_string(),
                service,
                code: None,
                routes: None,
                error: Some(format!("Failed to load project: {}", e)),
                inline_lambda_routes: 0,
            }
        }
    };

    let (routes, inline_lambda_routes) = match loader::load_routes(dir) {
        Ok(rt) => (Some(rt.0), rt.1),
        Err(e) => {
            // Keep the code graph even when route extraction fails: the call
            // graph is still useful, and the caller reports the gap.
            if let Some(cg) = code.take() {
                code = Some(cg);
            }
            return ProjectLoad {
                dir: dir.to_string(),
                service,
                code,
                routes: None,
                error: Some(format!("Failed to load routes: {}", e)),
                inline_lambda_routes: 0,
            };
        }
    };

    ProjectLoad {
        dir: dir.to_string(),
        service,
        code,
        routes,
        error: None,
        inline_lambda_routes,
    }
}

/// Load and merge code graphs + route tables from multiple comma-separated
/// project directories. Each project is annotated with its directory name as
/// service. Fails if any project cannot be loaded.
pub fn load_merged_projects(sources: &str) -> Result<(CodeGraph, RouteTable)> {
    let dirs = split_sources(sources)?;

    let mut merged_code = CodeGraph::new();
    let mut merged_routes = RouteTable::new();

    for dir in &dirs {
        let loaded = load_project(dir);
        if let Some(ref err) = loaded.error {
            anyhow::bail!("{}: {}", dir, err);
        }
        if let Some(cg) = loaded.code {
            merge_code(&mut merged_code, cg);
        }
        if let Some(rt) = loaded.routes {
            merge_routes(&mut merged_routes, rt);
        }
    }

    Ok((merged_code, merged_routes))
}

/// Split and validate the comma-separated `--sources` argument.
pub fn split_sources(sources: &str) -> Result<Vec<String>> {
    let dirs: Vec<String> = sources
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if dirs.is_empty() {
        anyhow::bail!("At least one source directory required");
    }
    Ok(dirs)
}

fn merge_code(into: &mut CodeGraph, cg: CodeGraph) {
    for m in cg.methods {
        into.add_method(m);
    }
    for c in cg.calls {
        if !into.calls.iter().any(|e| {
            e.source_class == c.source_class
                && e.source_method == c.source_method
                && e.target_class == c.target_class
                && e.target_method == c.target_method
        }) {
            into.calls.push(c);
        }
    }
    for cls in cg.classes {
        if !into.classes.contains(&cls) {
            into.classes.push(cls);
        }
    }
    for iface in cg.interfaces {
        if !into.interfaces.contains(&iface) {
            into.interfaces.push(iface);
        }
    }
}

fn merge_routes(into: &mut RouteTable, rt: RouteTable) {
    for e in rt.entries {
        if !into.entries.iter().any(|x| {
            x.http_method == e.http_method
                && x.route_pattern == e.route_pattern
                && x.class == e.class
                && x.method == e.method
        }) {
            into.add(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_unified_invalid_paths() {
        let result = build_unified("/nonexistent/trace.json", "/nonexistent/source");
        assert!(result.is_err());
    }
}
