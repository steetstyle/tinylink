use anyhow::{Context, Result};

use crate::sdg::otel;
use crate::code::loader;
use crate::link::types::{UnifiedGraph, UnifiedEdge};
use crate::link::mapper;

/// Build a UnifiedGraph from an OTLP trace file and C# source directory
pub fn build_unified(trace_path: &str, source_dir: &str) -> Result<UnifiedGraph> {
    let sdg = otel::parse_trace_file(trace_path)
        .with_context(|| format!("Failed to parse trace: {}", trace_path))?;

    let code_graph = loader::load_project(source_dir)
        .with_context(|| format!("Failed to load project: {}", source_dir))?;

    let route_table = loader::load_routes(source_dir)
        .with_context(|| format!("Failed to extract routes from: {}", source_dir))?;

    let unified = mapper::build_unified_from_sdg(&sdg, &code_graph, &route_table);

    Ok(unified)
}

/// Build a trace path: walk the UnifiedGraph to show span-by-span flow
pub fn build_trace_path(
    trace_path: &str, source_dir: &str, filter_trace_id: Option<&str>,
) -> Result<Vec<TraceStep>> {
    let unified = build_unified(trace_path, source_dir)?;
    let mut steps = Vec::new();

    for edge in &unified.edges {
        if let Some(tid) = filter_trace_id {
            if edge.span_id.as_deref() != Some(tid) {
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
            span_id: edge.span_id.clone(),
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
    pub span_id: Option<String>,
}

/// Query error paths in the unified graph
pub fn query_errors(
    trace_path: &str, source_dir: &str, service_filter: Option<&str>,
) -> Result<Vec<UnifiedEdge>> {
    let unified = build_unified(trace_path, source_dir)?;

    let errors: Vec<_> = unified.edges.into_iter()
        .filter(|e| e.error)
        .filter(|e| {
            if let Some(ref svc) = service_filter {
                e.source_service == *svc || e.target_service == *svc
            } else {
                true
            }
        })
        .collect();

    Ok(errors)
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
