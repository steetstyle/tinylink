use std::path::Path;
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

// ---------------------------------------------------------------------------
// PDG
// ---------------------------------------------------------------------------

/// Generate PDG (Program Dependence Graph) DOT for methods in C# files.
/// When --class and --method are given, filter to only that method's nodes.
pub fn handle_pdg(
    source_dir: &str,
    class_filter: &str,
    method_filter: &str,
    output: Option<&str>,
) -> Result<()> {
    use tiny_pdg_cs::cfg::builder::build_cfg;
    use tiny_pdg_cs::pdg::pdg_builder::build_pdg;
    use petgraph::visit::EdgeRef;
    use crate::code::loader::collect_cs_files;

    let path = Path::new(source_dir);
    let mut all_cs_files: Vec<String> = Vec::new();
    collect_cs_files(path, &mut all_cs_files);

    if all_cs_files.is_empty() {
        anyhow::bail!("No .cs files found in {}", source_dir);
    }

    let has_filter = !class_filter.is_empty() && !method_filter.is_empty();

    let mut dot = String::from("digraph PDG {\n  rankdir=TB;\n  node [shape=box style=rounded];\n\n");
    let mut any_output = false;

    for file_path in &all_cs_files {
        let source = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read: {}", file_path))?;

        // If filtering, find the target method's line range
        let method_line_range = if has_filter {
            find_method_line_range(&source, class_filter, method_filter)
        } else {
            None
        };

        // Check this file contains the target method when filtering
        if has_filter && method_line_range.is_none() {
            continue;
        }

        let cfg = match build_cfg(&source) {
            Ok(cfg) => cfg,
            Err(e) => {
                if !has_filter {
                    eprintln!("  ⚠ CFG failed for {}: {}", file_path, e);
                }
                continue;
            }
        };

        if cfg.node_count() == 0 {
            continue;
        }

        let pdg = match build_pdg(&cfg) {
            Ok(pdg) => pdg,
            Err(e) => {
                if !has_filter {
                    eprintln!("  ⚠ PDG failed for {}: {}", file_path, e);
                }
                continue;
            }
        };

        let file_label = file_path.replace(source_dir, "").trim_start_matches('/')
            .replace('"', "'");

        let subgraph_name = if has_filter {
            format!("{}: {}.{}", file_label, class_filter, method_filter)
        } else {
            file_label.clone()
        };

        dot.push_str(&format!("  subgraph cluster_{} {{\n", hash_name(&subgraph_name)));
        dot.push_str(&format!("    label=\"{}\";\n    labelloc=t;\n\n", subgraph_name));

        // Determine which nodes to include
        let include_node: Box<dyn Fn(usize, &tiny_pdg_cs::cfg::builder::BasicBlock) -> bool> =
            if let Some((ml_start, ml_end)) = method_line_range {
                Box::new(move |_, b| b.start_line >= ml_start && b.end_line <= ml_end)
            } else {
                Box::new(|_, _| true)
            };

        let node_indices: Vec<_> = pdg.node_indices()
            .filter(|&idx| include_node(idx.index(), &pdg[idx]))
            .collect();

        if node_indices.is_empty() {
            dot.push_str("  }\n\n");
            continue;
        }

        any_output = true;

        for &idx in &node_indices {
            let block = &pdg[idx];
            let label = format!("L{}-{}:{}", block.start_line, block.end_line, block.kind)
                .replace('"', "'");
            let color = match block.kind {
                tiny_pdg_cs::cfg::builder::BlockKind::Entry => "green",
                tiny_pdg_cs::cfg::builder::BlockKind::Exit => "red",
                _ => "lightblue",
            };
            dot.push_str(&format!("    n{} [label=\"{}\" style=filled fillcolor={}];\n",
                idx.index(), label, color));
        }

        dot.push_str("\n");
        for e in pdg.edge_references() {
            let src_include = include_node(e.source().index(), &pdg[e.source()]);
            let tgt_include = include_node(e.target().index(), &pdg[e.target()]);
            if !src_include || !tgt_include {
                continue;
            }
            let style = match e.weight() {
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Control => "dashed",
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Data => "dotted",
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Cfg(_) => "solid",
            };
            let color = match e.weight() {
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Control => "red",
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Data => "blue",
                tiny_pdg_cs::pdg::pdg_builder::PdgEdge::Cfg(_) => "black",
            };
            dot.push_str(&format!("    n{} -> n{} [style={} color={}];\n",
                e.source().index(), e.target().index(), style, color));
        }

        dot.push_str("  }\n\n");
    }

    dot.push_str("}\n");

    if !any_output && has_filter {
        anyhow::bail!("Method {}.{} not found in {}", class_filter, method_filter, source_dir);
    }

    if let Some(out_path) = output {
        std::fs::write(out_path, &dot)
            .with_context(|| format!("Failed to write DOT to {}", out_path))?;
        eprintln!("PDG written to {}", out_path);
    } else {
        println!("{}", dot);
    }

    Ok(())
}

/// Find a method's line range by scanning C# source text for class/method declarations.
fn find_method_line_range(source: &str, class: &str, method: &str) -> Option<(usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if !line.trim().contains(&format!("class {}", class)) {
            continue;
        }
        let mut brace_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;

        for (j, scan) in lines.iter().enumerate().skip(i + 1) {
            let opens = scan.matches('{').count() as i32;
            let closes = scan.matches('}').count() as i32;
            brace_depth += opens - closes;

            let st = scan.trim();
            if st.contains(&format!("{}(", method)) || st.contains(&format!("{} (", method)) {
                let mut m_depth = opens - closes;
                let mut saw_open = m_depth > 0;
                for (k, inner) in lines.iter().enumerate().skip(j + 1) {
                    let so = inner.matches('{').count() as i32;
                    let sc = inner.matches('}').count() as i32;
                    m_depth += so - sc;
                    if so > 0 { saw_open = true; }
                    if saw_open && m_depth <= 0 {
                        return Some((j + 1, k + 1));
                    }
                }
                return Some((j + 1, lines.len()));
            }

            if brace_depth <= 0 && j > i + 1 {
                break;
            }
        }
        return None;
    }
    None
}

fn hash_name(s: &str) -> String {
    let h: u64 = s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("_{:x}", h)
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

/// Interactive cross-service exploration:
/// Load SDG (OTEL or Prometheus) + multiple code projects → show unified view
/// Navigate: services → endpoints → code methods → callees/callers
pub fn handle_explore(source: &str, sources: &str, from: Option<&str>, show_pdg: bool) -> Result<()> {
    use crate::sdg::otel;
    use crate::code::loader::{load_routes, load_project};
    use crate::link::mapper;

    let dirs: Vec<&str> = sources.split(',').map(|s| s.trim()).collect();
    if dirs.is_empty() {
        anyhow::bail!("At least one source directory required");
    }

    // 1. Load SDG
    let sdg = match from.as_deref().unwrap_or("otel") {
        "prometheus" => {
            #[cfg(not(feature = "online"))]
            anyhow::bail!("Prometheus requires `online` feature");
            #[cfg(feature = "online")]
            {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::sdg::prom::query_prometheus(source, "5m"))?
            }
        }
        _ => otel::parse_trace_file(source)?,
    };
    println!("═══ Service Dependency Graph ═══");
    println!("  {} services, {} edges", sdg.nodes.len(), sdg.edges.len());
    for (name, node) in &sdg.nodes {
        println!("  {} [{} endpoints, {} errors]",
            name, node.endpoints.len(), node.error_count);
    }

    // 2. Load code + routes for each service directory
    let mut merged_code = crate::code::types::CodeGraph::new();
    let mut merged_routes = crate::code::route::RouteTable::new();

    for dir in &dirs {
        let svc_name = Path::new(dir).file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match load_project(dir) {
            Ok(mut cg) => {
                for m in &mut cg.methods {
                    m.service = Some(svc_name.clone());
                }
                for m in cg.methods {
                    merged_code.add_method(m);
                }
                merged_code.calls.extend(cg.calls);
                println!("  Code: {} loaded ({} methods, {} calls in merged)",
                    svc_name, merged_code.methods.len(), merged_code.calls.len());
            }
            Err(e) => println!("  ⚠ {}: {}", svc_name, e),
        }
        match load_routes(dir) {
            Ok(rt) => {
                let count = rt.entries.len();
                for e in rt.entries {
                    merged_routes.add(e);
                }
                println!("  Routes: {} loaded ({} entries)", svc_name, count);
            }
            Err(e) => println!("  ⚠ {} routes: {}", svc_name, e),
        }
    }

    // 3. Build UnifiedGraph
    let unified = mapper::build_unified_from_sdg(&sdg, &merged_code, &merged_routes);

    // Determine which SDG services had code resolved by the mapper.
    // A service is "coded" if at least one method was resolved for it.
    let mut coded_services: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in &unified.edges {
        if e.source_method.is_some() {
            coded_services.insert(e.source_service.clone());
        }
        if e.target_method.is_some() {
            coded_services.insert(e.target_service.clone());
        }
    }

    // Filter: only show edges where at least one service had code resolution
    let edges: Vec<_> = unified.edges.iter()
        .filter(|e| coded_services.contains(&e.source_service) || coded_services.contains(&e.target_service))
        .collect();

    println!("\n═══ Unified Graph ═══");
    println!("  {} coded services, {} edges", coded_services.len(), edges.len());

    // 4. Print service → endpoint → method mapping
    println!("\n═══ Service → Endpoint → Method ═══");
    for edge in &edges {
        let src_method = edge.source_method.as_deref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "(external)".into());
        let tgt_method = edge.target_method.as_deref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "(external)".into());
        let src_tag = if coded_services.contains(&edge.source_service) { "" } else { " [external]" };
        let tgt_tag = if coded_services.contains(&edge.target_service) { "" } else { " [external]" };
        println!("  {}{} [{}]", edge.source_service, src_tag, src_method);
        println!("    → {}{} [{}]", edge.target_service, tgt_tag, tgt_method);
        if let Some(ref f) = edge.target_file {
            println!("      file: {}:{}", f, edge.target_line.unwrap_or(0));
        }
        if edge.error {
            println!("      ⚠ ERROR (latency: {:.0}ms)", edge.latency_ms);
        }
    }

    if edges.is_empty() {
        println!("  (no edges involve coded services)");
    }

    // 5. Cross-service call summary (only fully coded pairs)
    println!("\n═══ Cross-Service Call Map ═══");
    use std::collections::BTreeSet;
    let mut pairs: Vec<(String, String)> = edges.iter()
        .filter(|e| coded_services.contains(&e.source_service) && coded_services.contains(&e.target_service))
        .map(|e| (e.source_service.clone(), e.target_service.clone()))
        .collect();
    pairs.sort();
    let unique: BTreeSet<_> = pairs.into_iter().collect();
    for (src, tgt) in &unique {
        println!("  {} → {}", src, tgt);
    }
    if unique.is_empty() {
        println!("  (no fully coded cross-service calls)");
    }

    // 6. Code-level traverse: for each resolved method in coded services, show call graph
    println!("\n═══ Code-Level Call Graph (matched methods) ═══");
    let mut methods_seen = std::collections::HashSet::new();
    for edge in &edges {
        for method_opt in [&edge.source_method, &edge.target_method] {
            if let Some(full) = method_opt {
                if methods_seen.contains(full) { continue; }
                methods_seen.insert(full.clone());
                let parts: Vec<&str> = full.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let cls = parts[0];
                    let mtd = parts[1];
                    if let Some(mn) = merged_code.find_method(cls, mtd) {
                        let caller_count = merged_code.calls.iter()
                            .filter(|c| c.target_class == cls && c.target_method == mtd)
                            .count();
                        let callee_count = merged_code.calls.iter()
                            .filter(|c| c.source_class == cls && c.source_method == mtd)
                            .count();
                        println!("  {:<40}  ↑callers:{}  ↓callees:{}  {}:{}",
                            full, caller_count, callee_count,
                            mn.file, mn.line_start);
                    }
                }
            }
        }
    }

    // 7. Drill-down: show PDG for each errored edge's target method
    if show_pdg {
        println!("\n═══ PDG Drill-Down (errored methods) ═══");
        for edge in &edges {
            if !edge.error {
                continue;
            }
            if let Some(ref method) = edge.target_method {
                let parts: Vec<&str> = method.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let class = parts[0];
                    let method_name = parts[1];
                    let tmp = format!("/tmp/pdg_explore_{}_{}.dot", class, method_name);
                    if handle_pdg(
                        &dirs.iter().find(|d| Path::new(d).file_name().map_or(false, |f| {
                            let svc = edge.target_service.replace(".", "").to_lowercase();
                            f.to_string_lossy().to_lowercase().contains(&svc)
                        })).unwrap_or(&dirs[0]),
                        class, method_name, Some(&tmp),
                    ).is_ok() {
                        if let Ok(content) = std::fs::read_to_string(&tmp) {
                            let lines: Vec<&str> = content.lines().collect();
                            println!("  PDG for {} [error]:", method);
                            for line in lines.iter().take(20) {
                                println!("    {}", line);
                            }
                            if lines.len() > 20 {
                                println!("    ... ({} lines total)", lines.len());
                            }
                        }
                        let _ = std::fs::remove_file(&tmp);
                    } else {
                        println!("  (PDG unavailable for {}: CFG build may have failed)", method);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn handle_query(trace: &str, source: &str, service: Option<&str>, show_pdg: bool) -> Result<()> {
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

        // Drill-down: show PDG for the errored method
        if show_pdg {
            if let Some(ref method) = edge.target_method {
                let parts: Vec<&str> = method.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let class = parts[0];
                    let method_name = parts[1];
                    println!("\n  ── PDG for {}.{} ──", class, method_name);
                    // Write PDG to a temp file so it's captured
                    let tmp = format!("/tmp/__pdg_{}_{}.dot", class, method_name);
                    if handle_pdg(source, class, method_name, Some(&tmp)).is_ok() {
                        if let Ok(content) = std::fs::read_to_string(&tmp) {
                            for line in content.lines().take(30) {
                                println!("  {}", line);
                            }
                            if content.lines().count() > 30 {
                                println!("  ... ({} lines total)", content.lines().count());
                            }
                        }
                        let _ = std::fs::remove_file(&tmp);
                    }
                    println!("  ──────────────────\n");
                }
            }
        }
    }

    Ok(())
}
