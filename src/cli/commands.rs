use anyhow::{Context, Result};
use std::path::Path;

use crate::code::loader;
use crate::link::{builder, index};
use crate::sdg::otel;

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

pub fn handle_code(source: &str, service: Option<&str>, format: Option<&str>) -> Result<()> {
    let mut code_graph = loader::load_project(source)
        .with_context(|| format!("Failed to load project: {}", source))?;

    // Annotate methods with service name derived from directory
    let svc_name = service.map(|s| s.to_string()).unwrap_or_else(|| {
        std::path::Path::new(source)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });
    for m in &mut code_graph.methods {
        m.service = Some(svc_name.clone());
    }

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

pub fn handle_link(trace: &str, sources: &str, format: Option<&str>) -> Result<()> {
    let unified = builder::build_unified(trace, sources)?;

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

    // Summary goes to stderr: `--format json` output must stay pipeable to jq.
    // It used to land on stdout and made the emitted JSON invalid.
    eprintln!(
        "Unified: {} nodes, {} edges, {} services, {} error traces",
        unified.nodes.len(),
        unified.edges.len(),
        unified.services.len(),
        unified.error_traces,
    );

    Ok(())
}

pub fn handle_tracepath(trace: &str, sources: &str, trace_id: Option<&str>) -> Result<()> {
    let steps = builder::build_trace_path(trace, sources, trace_id)?;

    if steps.is_empty() {
        println!("No trace path found.");
        return Ok(());
    }

    println!("Trace Path ({} steps):", steps.len());
    println!(
        "{:<6} {:<20} {:<30} {:<20} {:<8} {:<8}",
        "#", "Source", "→ Target", "Method", "Error", "Latency"
    );
    println!("{}", "-".repeat(100));

    for (i, step) in steps.iter().enumerate() {
        let _src_method = step.source_method.as_deref().unwrap_or("?");
        let tgt_method = step.target_method.as_deref().unwrap_or("?");
        // Filter empty strings: an unknown file rendered as ":14".
        let file_loc = step
            .target_file
            .as_deref()
            .filter(|f| !f.is_empty())
            .unwrap_or("?");
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
// Index
// ---------------------------------------------------------------------------

/// Build or reuse a SQLite index of the code, routes and call edges.
///
/// The other commands reparse every `.cs` file on each run, which is fine for a
/// one-shot call and far too slow for a long-lived process such as an MCP server.
/// This writes the parsed result to a file that later runs can read.
///
/// Skips the rebuild when the sources are byte-for-byte unchanged, so this is
/// cheap to call from CI on every commit.
pub fn handle_index(sources: &str, out: &str, force: bool, dry_run: bool) -> Result<()> {
    let dirs = builder::split_sources(sources)?;
    let out_path = std::path::Path::new(out);

    let wanted_hash = index::content_hash(&dirs)?;

    if !force {
        if let Ok(conn) = index::open(out_path) {
            if index::check_schema(&conn).is_ok() {
                if let Some(existing) = index::stored_hash(&conn)? {
                    if existing == wanted_hash {
                        // Summary goes to stderr so stdout stays clean for
                        // scripts that capture it.
                        eprintln!(
                            "Index is up to date ({}); use --force to rebuild.",
                            out_path.display()
                        );
                        return Ok(());
                    }
                }
            }
        }
    }

    let index = index::build(&dirs)?;

    for warning in &index.warnings {
        eprintln!("  ⚠ {}", warning);
    }

    eprintln!(
        "Indexed {} methods, {} routes, {} call edges across {} project(s)",
        index.code.methods.len(),
        index.routes.entries.len(),
        index.code.calls.len(),
        dirs.len()
    );
    if index.inline_lambda_routes > 0 {
        eprintln!(
            "  ⚠ {} route(s) use an inline lambda and have no named handler",
            index.inline_lambda_routes
        );
    }

    if dry_run {
        eprintln!("Dry run: nothing written.");
        return Ok(());
    }

    index::write(&index, out_path)?;
    let size = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "Wrote {} ({:.1} KiB), content hash {}",
        out_path.display(),
        size as f64 / 1024.0,
        &index.content_hash[..12.min(index.content_hash.len())]
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// PDG
// ---------------------------------------------------------------------------

/// Predicate deciding whether a PDG basic block belongs in the output.
type NodeFilter = Box<dyn Fn(usize, &tiny_pdg_cs::cfg::builder::BasicBlock) -> bool>;

/// Generate PDG (Program Dependence Graph) DOT for methods in C# files.
/// When --class and --method are given, filter to only that method's nodes.
pub fn handle_pdg(
    source_dir: &str,
    class_filter: &str,
    method_filter: &str,
    output: Option<&str>,
) -> Result<()> {
    use crate::code::loader::collect_cs_files;
    use petgraph::visit::EdgeRef;
    use tiny_pdg_cs::cfg_builder::build_cfg;

    let path = Path::new(source_dir);
    let mut all_cs_files: Vec<String> = Vec::new();
    collect_cs_files(path, &mut all_cs_files);

    if all_cs_files.is_empty() {
        anyhow::bail!("No .cs files found in {}", source_dir);
    }

    let has_filter = !class_filter.is_empty() && !method_filter.is_empty();

    let mut dot =
        String::from("digraph PDG {\n  rankdir=TB;\n  node [shape=box style=rounded];\n\n");
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

        let pdg = match tiny_pdg_cs::pdg::pdg_builder::build_pdg(&cfg) {
            Ok(pdg) => pdg,
            Err(e) => {
                if !has_filter {
                    eprintln!("  ⚠ PDG failed for {}: {}", file_path, e);
                }
                continue;
            }
        };

        let file_label = file_path
            .replace(source_dir, "")
            .trim_start_matches('/')
            .replace('"', "'");

        let subgraph_name = if has_filter {
            format!("{}: {}.{}", file_label, class_filter, method_filter)
        } else {
            file_label.clone()
        };

        // Determine which nodes to include
        let include_node: NodeFilter = if let Some((ml_start, ml_end)) = method_line_range {
            Box::new(move |_, b| b.start_line >= ml_start && b.end_line <= ml_end)
        } else {
            Box::new(|_, _| true)
        };

        let filtered_indices: Vec<_> = pdg
            .node_indices()
            .filter(|&idx| include_node(idx.index(), &pdg[idx]))
            .collect();

        if filtered_indices.is_empty() {
            continue;
        }

        any_output = true;

        // Build a filtered subgraph with only the selected nodes
        use petgraph::graph::DiGraph;
        use tiny_pdg_cs::cfg::builder::BasicBlock;
        use tiny_pdg_cs::pdg::pdg_builder::PdgEdge;
        let mut sub = DiGraph::<BasicBlock, PdgEdge>::new();
        let mut mapping = std::collections::HashMap::new();
        for &idx in &filtered_indices {
            let new_idx = sub.add_node(pdg[idx].clone());
            mapping.insert(idx, new_idx);
        }
        for e in pdg.edge_references() {
            if let (Some(&s), Some(&t)) = (mapping.get(&e.source()), mapping.get(&e.target())) {
                sub.add_edge(s, t, e.weight().clone());
            }
        }

        dot.push_str(&format!(
            "  subgraph cluster_{} {{\n",
            hash_name(&subgraph_name)
        ));
        dot.push_str(&format!(
            "    label=\"{}\";\n    labelloc=t;\n\n",
            subgraph_name
        ));
        dot.push_str(&tiny_pdg_cs::dot::pdg_nodes_edges_to_dot(&sub, "    "));
        dot.push_str("  }\n\n");
    }

    dot.push_str("}\n");

    if !any_output && has_filter {
        anyhow::bail!(
            "Method {}.{} not found in {}",
            class_filter,
            method_filter,
            source_dir
        );
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

/// Find a method's line range using the C# syntax tree.
///
/// The previous text scan looked for `MethodName(` anywhere on a line, so a
/// *call site* preceded the declaration — `GetItemById(5)` inside another method
/// — and the PDG drill-down then showed that caller's statement instead of the
/// target method, silently. Walking `method_declaration` nodes and reading each
/// one's `name` field identifies the actual declaration. Generic and async
/// signatures are covered because the parser, not a regex, splits them.
fn find_method_line_range(source: &str, class: &str, method: &str) -> Option<(usize, usize)> {
    use tiny_pdg_cs::parse::parser::parse_source;

    let tree = parse_source(source).ok()?;
    let root = tree.root_node();

    // Collect (class_name, start_row, end_row) for every method declaration.
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        for child in children {
            if child.kind() == "method_declaration" {
                let name = child
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok());
                if let Some(name) = name {
                    if name == method && method_in_class(child, class, source) {
                        // tree-sitter rows are 0-based; PDG block lines are 1-based.
                        return Some((
                            child.start_position().row + 1,
                            child.end_position().row + 1,
                        ));
                    }
                }
            }
            stack.push(child);
        }
    }
    None
}

/// Is this method declaration nested inside the named class (or a base class)?
fn method_in_class(method_node: tree_sitter::Node, class: &str, source: &str) -> bool {
    let mut current = method_node.parent();
    while let Some(n) = current {
        if matches!(
            n.kind(),
            "class_declaration" | "struct_declaration" | "record_declaration"
        ) {
            if let Some(name) = n
                .child_by_field_name("name")
                .and_then(|x| x.utf8_text(source.as_bytes()).ok())
            {
                // Accept the class itself or any enclosing base, so a method
                // inherited into a derived controller still resolves.
                return name == class;
            }
        }
        current = n.parent();
    }
    false
}

fn hash_name(s: &str) -> String {
    let h: u64 = s
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("_{:x}", h)
}

#[cfg(feature = "online")]
pub fn handle_prometheus(url: &str, since: &str, format: Option<&str>) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    let graph = rt.block_on(crate::sdg::prom::query_prometheus(url, since))?;

    match format.unwrap_or("json") {
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
pub fn handle_explore(
    source: &str,
    sources: &str,
    from: Option<&str>,
    show_pdg: bool,
) -> Result<()> {
    use crate::link::mapper;
    use crate::sdg::otel;

    let dirs: Vec<&str> = sources.split(',').map(|s| s.trim()).collect();
    if dirs.is_empty() {
        anyhow::bail!("At least one source directory required");
    }

    // 1. Load SDG
    let sdg = match from.unwrap_or("otel") {
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
        println!(
            "  {} [{} endpoints, {} errors]",
            name,
            node.endpoints.len(),
            node.error_count
        );
    }

    // 2. Load code + routes for each service directory
    let mut merged_code = crate::code::types::CodeGraph::new();
    let mut merged_routes = crate::code::route::RouteTable::new();
    let mut inline_lambda_routes = 0usize;

    for dir in &dirs {
        let loaded = builder::load_project(dir);
        let svc_name = loaded.service.clone();
        if let Some(ref err) = loaded.error {
            println!("  ⚠ {}", err);
        }
        if let Some(cg) = loaded.code {
            merge_code_graph(&mut merged_code, cg);
            println!(
                "  Code: {} loaded ({} methods, {} calls in merged)",
                svc_name,
                merged_code.methods.len(),
                merged_code.calls.len()
            );
        }
        if let Some(rt) = loaded.routes {
            let count = rt.entries.len();
            merge_route_table(&mut merged_routes, rt);
            println!("  Routes: {} loaded ({} entries)", svc_name, count);
        }
        if loaded.inline_lambda_routes > 0 {
            println!(
                "  ⚠ {}: {} route(s) use an inline lambda and have no named handler",
                svc_name, loaded.inline_lambda_routes
            );
            inline_lambda_routes += loaded.inline_lambda_routes;
        }
    }
    if inline_lambda_routes > 0 {
        println!(
            "  ({} route(s) total cannot be mapped to a method)",
            inline_lambda_routes
        );
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
    let edges: Vec<_> = unified
        .edges
        .iter()
        .filter(|e| {
            coded_services.contains(&e.source_service) || coded_services.contains(&e.target_service)
        })
        .collect();

    println!("\n═══ Unified Graph ═══");
    println!(
        "  {} coded services, {} edges",
        coded_services.len(),
        edges.len()
    );

    // 4. Print service → endpoint → method mapping
    println!("\n═══ Service → Endpoint → Method ═══");
    for edge in &edges {
        let src_method = edge
            .source_method
            .as_deref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "(external)".into());
        let tgt_method = edge
            .target_method
            .as_deref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "(external)".into());
        let src_tag = if coded_services.contains(&edge.source_service) {
            ""
        } else {
            " [external]"
        };
        let tgt_tag = if coded_services.contains(&edge.target_service) {
            ""
        } else {
            " [external]"
        };
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
    let mut pairs: Vec<(String, String)> = edges
        .iter()
        .filter(|e| {
            coded_services.contains(&e.source_service) && coded_services.contains(&e.target_service)
        })
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
        for full in [&edge.source_method, &edge.target_method]
            .into_iter()
            .flatten()
        {
            if methods_seen.contains(full) {
                continue;
            }
            methods_seen.insert(full.clone());
            let parts: Vec<&str> = full.splitn(2, '.').collect();
            if parts.len() == 2 {
                let cls = parts[0];
                let mtd = parts[1];
                if let Some(mn) = merged_code.find_method(cls, mtd) {
                    let caller_count = merged_code
                        .calls
                        .iter()
                        .filter(|c| c.target_class == cls && c.target_method == mtd)
                        .count();
                    let callee_count = merged_code
                        .calls
                        .iter()
                        .filter(|c| c.source_class == cls && c.source_method == mtd)
                        .count();
                    println!(
                        "  {:<40}  ↑callers:{}  ↓callees:{}  {}:{}",
                        full, caller_count, callee_count, mn.file, mn.line_start
                    );
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
                        dirs.iter()
                            .find(|d| {
                                Path::new(d).file_name().is_some_and(|f| {
                                    let svc = edge.target_service.replace(".", "").to_lowercase();
                                    f.to_string_lossy().to_lowercase().contains(&svc)
                                })
                            })
                            .unwrap_or(&dirs[0]),
                        class,
                        method_name,
                        Some(&tmp),
                    )
                    .is_ok()
                    {
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
                        println!(
                            "  (PDG unavailable for {}: CFG build may have failed)",
                            method
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Merge a loaded project code graph into an accumulator, de-duplicating calls.
fn merge_code_graph(into: &mut crate::code::types::CodeGraph, cg: crate::code::types::CodeGraph) {
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

/// Merge a loaded route table into an accumulator, de-duplicating entries.
fn merge_route_table(
    into: &mut crate::code::route::RouteTable,
    rt: crate::code::route::RouteTable,
) {
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

/// Find the source directory that most likely corresponds to a service name.
fn find_source_dir_for_service(sources: &str, service: &str) -> String {
    let svc_clean = service.replace('.', "").to_lowercase();
    for dir in sources.split(',').map(|s| s.trim()) {
        if let Some(name) = Path::new(dir).file_name() {
            let n = name.to_string_lossy().to_lowercase();
            if n.contains(&svc_clean) || svc_clean.contains(&n) {
                return dir.to_string();
            }
        }
    }
    // fallback: first directory
    sources
        .split(',')
        .map(|s| s.trim())
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn handle_query(
    trace: &str,
    sources: &str,
    service: Option<&str>,
    show_pdg: bool,
) -> Result<()> {
    let errors = builder::query_errors(trace, sources, service)?;

    if errors.is_empty() {
        println!("No errors found.");
        return Ok(());
    }

    println!("Error Paths ({} edge(s)):", errors.len());
    println!(
        "{:<6} {:<20} {:<20} {:<30} {:<8}",
        "#", "From", "To", "Method", "File:Line"
    );
    println!("{}", "-".repeat(90));

    for (i, edge) in errors.iter().enumerate() {
        let tgt_method = edge.target_method.as_deref().unwrap_or("?");
        let file_loc = edge
            .target_file
            .as_deref()
            .filter(|f| !f.is_empty())
            .unwrap_or("?");
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
                    let dir = find_source_dir_for_service(sources, &edge.target_service);
                    if handle_pdg(&dir, class, method_name, Some(&tmp)).is_ok() {
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
