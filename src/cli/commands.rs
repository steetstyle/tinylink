use anyhow::{Context, Result};

use crate::sdg::otel;
use crate::code::loader;
use crate::link::builder;

pub fn handle_sdg(trace: &str, format: Option<&str>, stats: bool) -> Result<()> {
    let graph = otel::parse_trace_file(trace)
        .with_context(|| format!("Failed to parse trace: {}", trace))?;

    let fmt = format.unwrap_or("json");
    match fmt {
        "json" => {
            let output = serde_json::to_string_pretty(&graph)?;
            println!("{}", output);
        }
        "dot" => {
            println!("{}", graph.to_dot());
        }
        _ => anyhow::bail!("Unsupported format: {}. Use json or dot.", fmt),
    }

    if stats {
        println!(
            "Services: {} | Edges: {} | Traces: {} | Errors: {}",
            graph.nodes.len(),
            graph.edges.len(),
            graph.total_traces,
            graph.error_traces,
        );
    }

    Ok(())
}

pub fn handle_code(source: &str, _service: Option<&str>, format: Option<&str>) -> Result<()> {
    let code_graph = loader::load_project(source)
        .with_context(|| format!("Failed to load project: {}", source))?;

    let fmt = format.unwrap_or("json");
    match fmt {
        "json" => {
            let output = serde_json::to_string_pretty(&code_graph)?;
            println!("{}", output);
        }
        "dot" => {
            println!("{}", code_graph.to_dot());
        }
        _ => anyhow::bail!("Unsupported format: {}. Use json or dot.", fmt),
    }

    Ok(())
}

pub fn handle_link(trace: &str, source: &str, format: Option<&str>) -> Result<()> {
    let unified = builder::build_unified(trace, source)?;

    let fmt = format.unwrap_or("json");
    match fmt {
        "json" => {
            let output = serde_json::to_string_pretty(&unified)?;
            println!("{}", output);
        }
        "dot" => {
            println!("{}", unified.to_dot());
        }
        _ => anyhow::bail!("Unsupported format: {}. Use json or dot.", fmt),
    }

    println!(
        "Unified: {} nodes, {} edges, {} services, {} error traces",
        unified.nodes.len(),
        unified.edges.len(),
        unified.services.len(),
        unified.error_traces,
    );

    Ok(())
}

pub fn handle_tracepath(trace: &str, source: &str, trace_id: Option<&str>) -> Result<()> {
    let steps = builder::build_trace_path(trace, source, trace_id)?;

    if steps.is_empty() {
        println!("No trace path found.");
        return Ok(());
    }

    println!("Trace Path ({} steps):", steps.len());
    println!("{:<6} {:<20} {:<30} {:<20} {:<8} {:<8}", "#", "Source", "→ Target", "Method", "Error", "Latency");
    println!("{}", "-".repeat(100));

    for (i, step) in steps.iter().enumerate() {
        let _src_method = step.source_method.as_deref().unwrap_or("?");
        let tgt_method = step.target_method.as_deref().unwrap_or("?");
        let file_loc = step.target_file.as_deref().unwrap_or("?");
        let line = step.target_line.unwrap_or(0);
        let err = if step.error { "⚠" } else { "✓" };
        println!(
            "  {:<3}  {:<18} → {:<28} {:<18} {:<6} {:<6.0}ms  {}:{}",
            i + 1,
            step.source_service,
            step.target_service,
            tgt_method,
            err,
            step.latency_ms,
            file_loc,
            line,
        );
    }

    Ok(())
}

#[cfg(feature = "online")]
pub fn handle_prometheus(url: &str, since: &str, format: Option<&str>) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?;
    let graph = rt.block_on(crate::sdg::prom::query_prometheus(url, since))?;

    match format.as_deref().unwrap_or("json") {
        "json" => {
            let output = serde_json::to_string_pretty(&graph)?;
            println!("{}", output);
        }
        "dot" => {
            println!("{}", graph.to_dot());
        }
        f => anyhow::bail!("Unsupported format: {}. Use json or dot.", f),
    }

    Ok(())
}

pub fn handle_query(trace: &str, source: &str, service: Option<&str>) -> Result<()> {
    let errors = builder::query_errors(trace, source, service)?;

    if errors.is_empty() {
        println!("No errors found.");
        return Ok(());
    }

    println!("Error Paths ({} edge(s)):", errors.len());
    println!("{:<6} {:<20} {:<20} {:<30} {:<8}", "#", "From", "To", "Method", "File:Line");
    println!("{}", "-".repeat(90));

    for (i, edge) in errors.iter().enumerate() {
        let tgt_method = edge.target_method.as_deref().unwrap_or("?");
        let file_loc = edge.target_file.as_deref().unwrap_or("?");
        let line = edge.target_line.unwrap_or(0);
        println!(
            "  {:<3}  {:<18} → {:<18} {:<28} {}:{}",
            i + 1,
            edge.source_service,
            edge.target_service,
            tgt_method,
            file_loc,
            line,
        );
    }

    Ok(())
}
