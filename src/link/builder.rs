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

/// Load one project directory, tagging its methods and routes with the service
/// name so lookups can be scoped to a service.
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
        Ok((mut rt, lambdas)) => {
            for e in &mut rt.entries {
                e.service = Some(service.clone());
            }
            (Some(rt), lambdas)
        }
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

/// Service name for a project directory: its final path component.
pub fn service_name_for_dir(dir: &str) -> String {
    std::path::Path::new(dir)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Split a directory into the project directories it contains.
///
/// A .NET solution root holds many services in subdirectories, and trace service
/// names refer to those subdirectories (`Agency.API`), not to the root. Treating
/// the root as one service tags every method with the solution's folder name, so
/// every service-scoped route lookup then fails and a correctly matched endpoint
/// degrades to an unscoped, lower-confidence guess.
///
/// A directory that is itself a project (it contains a `.csproj`) or a plain set
/// of `.cs` files is returned unchanged.
pub fn project_dirs_in(dir: &str) -> Vec<String> {
    let path = std::path::Path::new(dir);
    if !path.is_dir() {
        return vec![dir.to_string()];
    }
    // Already a project: load it whole.
    if has_csproj(path) {
        return vec![dir.to_string()];
    }

    let mut projects: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() && has_csproj(&child) {
                projects.push(child.to_string_lossy().to_string());
            }
        }
    }
    projects.sort();

    if projects.is_empty() {
        // Not a solution layout: a directory of loose .cs files, or a project
        // without a .csproj. Load it as one unit.
        vec![dir.to_string()]
    } else {
        projects
    }
}

fn has_csproj(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.path()
            .extension()
            .map(|x| x.eq_ignore_ascii_case("csproj"))
            .unwrap_or(false)
    })
}

/// Load and merge code graphs + route tables from multiple comma-separated
/// project directories. Each project is annotated with its directory name as
/// service. Fails if any project cannot be loaded.
///
/// A directory that is a solution root is expanded into its constituent projects
/// so that each service is tagged with its own name; see [`project_dirs_in`].
pub fn load_merged_projects(sources: &str) -> Result<(CodeGraph, RouteTable)> {
    let dirs = split_sources(sources)?;
    load_merged_project_dirs(&dirs)
}

/// Like [`load_merged_projects`] but takes resolved directories.
pub fn load_merged_project_dirs(dirs: &[String]) -> Result<(CodeGraph, RouteTable)> {
    let mut merged_code = CodeGraph::new();
    let mut merged_routes = RouteTable::new();

    for dir in expand_solution_roots(dirs) {
        let loaded = load_project(&dir);
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

/// Expand solution roots into their projects, keeping the order stable.
pub fn expand_solution_roots(dirs: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    for dir in dirs {
        for project in project_dirs_in(dir) {
            if !expanded.contains(&project) {
                expanded.push(project);
            }
        }
    }
    expanded
}

/// Split and validate the comma-separated `--sources` argument.
pub fn split_sources(sources: &str) -> Result<Vec<String>> {
    let specs: Vec<&str> = sources
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if specs.is_empty() {
        anyhow::bail!("At least one source directory required");
    }

    // A `gh:owner/repo@ref` entry is fetched here, once, and everything downstream
    // sees a directory. Putting it at this function rather than in the index command
    // means the CLI, the MCP tool and load_merged_projects all get it, and nothing
    // further has to know a specifier exists.
    let mut dirs = Vec::with_capacity(specs.len());
    for spec in specs {
        let checkout = tiny_pdg_cs::source::resolve(spec)
            .with_context(|| format!("resolving source '{spec}'"))?;
        tiny_pdg_cs::source::announce(spec);
        dirs.push(checkout.dir.to_string_lossy().into_owned());
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
        // Service is part of the identity: two services may legitimately declare
        // the same pattern, and collapsing them would lose one service's route.
        if !into.entries.iter().any(|x| {
            x.http_method == e.http_method
                && x.route_pattern == e.route_pattern
                && x.class == e.class
                && x.method == e.method
                && x.service == e.service
        }) {
            into.add(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `gh:` entry must come back as a directory, because everything downstream
    /// opens these as paths. Resolving it anywhere else would mean every caller had to
    /// know a specifier exists.
    #[test]
    fn a_github_specifier_resolves_to_a_directory() {
        // No network: the checkout is placed in the cache by hand, which is all
        // resolve() needs to take the cached path.
        let cache = std::env::temp_dir().join("tinylink_split_sources_test");
        let _ = std::fs::remove_dir_all(&cache);

        let previous = std::env::var_os("TINY_SOURCE_CACHE");
        unsafe { std::env::set_var("TINY_SOURCE_CACHE", &cache) };
        let key = tiny_pdg_cs::source::parse_github("gh:steetstyle/tinylink@main")
            .unwrap()
            .unwrap()
            .cache_dir();
        std::fs::create_dir_all(key.join(".git")).unwrap();
        let dirs = split_sources("gh:steetstyle/tinylink@main").expect("must resolve");
        unsafe {
            match previous {
                Some(value) => std::env::set_var("TINY_SOURCE_CACHE", value),
                None => std::env::remove_var("TINY_SOURCE_CACHE"),
            }
        }

        assert_eq!(dirs, vec![key.to_string_lossy().into_owned()]);
        assert!(std::path::Path::new(&dirs[0]).is_dir());
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn local_paths_and_specifiers_can_be_mixed() {
        // Two projects, one checked out and one on disk: the usual reason for a
        // comma-separated list, and it has to keep working.
        let dirs = split_sources("./does-not-matter, /also-not").expect("no fetch attempted");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], "./does-not-matter");
    }

    #[test]
    fn an_empty_list_is_still_refused() {
        assert!(split_sources("  ,  ").is_err());
        assert!(split_sources("").is_err());
    }

    #[test]
    fn test_build_unified_invalid_paths() {
        let result = build_unified("/nonexistent/trace.json", "/nonexistent/source");
        assert!(result.is_err());
    }
}
