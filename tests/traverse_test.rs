use std::collections::HashMap;
use std::sync::Arc;

/// Helper: collect .cs files recursively
fn collect_cs_files(dir: &std::path::Path, files: &mut Vec<String>) {
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

/// Helper: parse source into TypeGraph + CallGraph
fn parse_project(
    dir: &str,
) -> (
    tiny_pdg_cs::resolve::types::TypeGraph,
    tiny_pdg_cs::analysis::callgraph::CallGraph,
) {
    let mut all_cs_files = Vec::new();
    collect_cs_files(std::path::Path::new(dir), &mut all_cs_files);

    let mut type_graph = tiny_pdg_cs::resolve::types::TypeGraph::default();
    let mut call_graph = tiny_pdg_cs::analysis::callgraph::CallGraph {
        calls: vec![],
        class_callees: HashMap::new(),
        class_creations: HashMap::new(),
        method_calls: HashMap::new(),
    };

    for file_path in &all_cs_files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let tree = match tiny_pdg_cs::parse::parser::parse_source(&source) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let st =
            match tiny_pdg_cs::resolve::symbols::SymbolTable::from_ast(tree.root_node(), &source) {
                Ok(s) => s,
                Err(_) => continue,
            };
        let cg = tiny_pdg_cs::analysis::callgraph::CallGraphBuilder::build(
            tree.root_node(),
            &source,
            &st.type_graph,
        );

        for (name, class) in st.type_graph.classes {
            type_graph.classes.entry(name).or_insert(class);
        }
        for (name, iface) in st.type_graph.interfaces {
            type_graph.interfaces.entry(name).or_insert(iface);
        }
        call_graph.calls.extend(cg.calls);
        for (class, callees) in cg.class_callees {
            call_graph
                .class_callees
                .entry(class)
                .or_default()
                .extend(callees);
        }
        for (class, cre) in cg.class_creations {
            call_graph
                .class_creations
                .entry(class)
                .or_default()
                .extend(cre);
        }
        for (method, m_calls) in cg.method_calls {
            call_graph
                .method_calls
                .entry(method)
                .or_default()
                .extend(m_calls);
        }
    }

    (type_graph, call_graph)
}

// ─────────────────────────────────────────────────────────────────

/// Locates the eShop checkout these traversal tests need.
const ESHOP_DIR: &str = "/tmp/eshop/src/Catalog.API";

/// These tests require an external eShop clone and a `PdgContext` built from a
/// real path, so they cannot run from a clean checkout. They are `#[ignore]`d
/// rather than silently returning early: an early `return` reports as a pass,
/// which hid the fact that they never executed.
///
/// Run with:
///   git clone --depth 1 https://github.com/dotnet/eShopOnWeb.git /tmp/eshop
///   cargo test --test traverse_test -- --ignored
#[ignore = "requires an eShop clone at /tmp/eshop; run with --ignored"]
#[test]
fn test_traverse_eshop_catalog_start_down_up() {
    let dir = ESHOP_DIR;
    assert!(
        std::path::Path::new(dir).exists(),
        "eShop not found at {dir}; see the test module docs for setup"
    );

    let (type_graph, call_graph) = parse_project(dir);
    assert!(!type_graph.classes.is_empty(), "No classes parsed");
    assert!(!call_graph.calls.is_empty(), "No calls parsed");

    let _pdg = Arc::new(tiny_pdg_cs::analysis::pdg_context::PdgContext::build(
        std::path::Path::new(dir),
    ));

    let mut state = tiny_pdg_cs::traverse::types::TraversalState::init(
        &type_graph,
        &call_graph,
        "CatalogApi",
        Some("Functional test: navigate eShop Catalog API call graph".into()),
        Some(dir),
    )
    .expect("Failed to init traversal state");

    assert_eq!(
        state.current.class, "CatalogApi",
        "Should start at CatalogApi"
    );
    assert_eq!(
        state.current.method, "MapCatalogApi",
        "Should start at MapCatalogApi (first non-auto-gen method)"
    );

    use tiny_pdg_cs::traverse::engine::{step, Action};
    use tiny_pdg_cs::traverse::types::Judgment;

    // ═══ TRAVERSAL PATH ═══════════════════════════════════════
    // MapCatalogApi ↓→ GetItemById ↓→ CatalogContext ↓→ … ↑→ back

    println!("\n═══ Traversal: MapCatalogApi ═══");
    println!("  Callees of MapCatalogApi: {}", state.down.len());
    for e in &state.down {
        let kind = format!("{:?}", e.kind);
        let kind_short = kind.split('(').next().unwrap_or(&kind);
        println!(
            "    [{}] {}.{}  via: {}  [{:?}]  line: {}",
            e.idx,
            e.target.class,
            e.target.method,
            e.via,
            kind_short,
            e.line.unwrap_or(0)
        );
    }

    // Step 1: MapCatalogApi → DOWN via Delegate → GetItemById
    // Find the Delegate entry containing GetItemById in its handlers
    let get_item_info: Option<(usize, usize)> = state.down.iter().find_map(|e| {
        if let tiny_pdg_cs::traverse::types::EdgeKind::Delegate { ref handlers } = e.kind {
            handlers
                .iter()
                .position(|h| h == "GetItemById")
                .map(|sub| (e.idx, sub))
        } else {
            None
        }
    });
    assert!(
        get_item_info.is_some(),
        "GetItemById should be a delegate handler of MapCatalogApi"
    );
    let (entry_idx, handler_idx) = get_item_info.unwrap();

    step(
        &mut state,
        &Action::DownDispatch(entry_idx, handler_idx),
        &type_graph,
        &call_graph,
    );
    println!(
        "\n── ↓{}{} → {}.{} ──",
        entry_idx,
        (b'a' + handler_idx as u8) as char,
        state.current.class,
        state.current.method
    );
    assert_eq!(state.current.method, "GetItemById");

    // Step 2: GetItemById → DOWN → first callee into EF Core / catalog context
    println!("\n  Callees of GetItemById ({}):", state.down.len());
    for e in &state.down {
        let kind = format!("{:?}", e.kind);
        let kind_short = kind.split('(').next().unwrap_or(&kind);
        println!(
            "    [{}] {}.{}  via: {}  [{:?}]  line: {}",
            e.idx,
            e.target.class,
            e.target.method,
            e.via,
            kind_short,
            e.line.unwrap_or(0)
        );
    }

    // Navigate into the first non-External callee (typically CatalogContext or similar)
    let internal_idx = state
        .down
        .iter()
        .find(|e| !matches!(e.kind, tiny_pdg_cs::traverse::types::EdgeKind::External))
        .map(|e| e.idx);
    if let Some(idx) = internal_idx {
        step(&mut state, &Action::Down(idx), &type_graph, &call_graph);
        println!(
            "\n── ↓{} → {}.{} ──",
            idx, state.current.class, state.current.method
        );

        // Step 3: Navigate UP back to GetItemById
        let up_idx = state.up.first().map(|e| e.idx);
        if let Some(uidx) = up_idx {
            step(&mut state, &Action::Up(uidx), &type_graph, &call_graph);
            println!(
                "\n── ↑{} → {}.{} ──",
                uidx, state.current.class, state.current.method
            );
            assert_eq!(
                state.current.method, "GetItemById",
                "After ↑ should return to GetItemById"
            );
        }
    }

    // Step 4: Complete(Primary) with evidence
    step(
        &mut state,
        &Action::Complete(Judgment::Primary),
        &type_graph,
        &call_graph,
    );
    println!("\n── Complete(Primary) ──");

    // Step 5: Advance queue → next method in CatalogApi
    if !state.queue.is_empty() {
        let prev = state.current.method.clone();
        state.next_in_queue(&type_graph, &call_graph);
        println!(
            "\n── Queue next: {}.{} (was {}) ──",
            state.current.class, state.current.method, prev
        );
        assert_ne!(state.current.method, prev, "Queue should advance");
    }

    // ═══ HISTORY ═══════════════════════════════════════════════
    println!("\n═══ Traversal History ═══");
    for (i, h) in state.history.iter().enumerate() {
        println!(
            "  [{}] {}.{} ↦ {}",
            i, h.node.class, h.node.method, h.action
        );
    }
    println!("  Final: {}.{}", state.current.class, state.current.method);
    println!("  Queue: {} remaining", state.queue.len());
    assert!(
        state.history.len() >= 2,
        "Should have at least 2 history entries"
    );
}

/// Deep semantic navigation: MapCatalogApi → GetAllItemsV1 → CatalogContext → ToListAsync
#[ignore = "requires an eShop clone at /tmp/eshop; run with --ignored"]
#[test]
fn test_traverse_eshop_deep_navigation() {
    let dir = ESHOP_DIR;
    assert!(
        std::path::Path::new(dir).exists(),
        "eShop not found at {dir}; see the test module docs for setup"
    );

    let (type_graph, call_graph) = parse_project(dir);
    let _pdg = Arc::new(tiny_pdg_cs::analysis::pdg_context::PdgContext::build(
        std::path::Path::new(dir),
    ));

    let mut state = tiny_pdg_cs::traverse::types::TraversalState::init(
        &type_graph,
        &call_graph,
        "CatalogApi",
        Some("MapCatalogApi → GetAllItemsV1 → CatalogContext → ToListAsync".into()),
        Some(dir),
    )
    .expect("Failed to init traversal");

    use tiny_pdg_cs::traverse::engine::{step, Action};
    use tiny_pdg_cs::traverse::types::Judgment;

    // ── Path: MapCatalogApi → GetAllItemsV1 → CatalogContext → method calls ──

    println!("\n═══ Deep Traversal ═══");

    // Step 1: Find GetAllItemsV1 as delegate handler
    let get_all_v1_info: Option<(usize, usize)> = state.down.iter().find_map(|e| {
        if let tiny_pdg_cs::traverse::types::EdgeKind::Delegate { ref handlers } = e.kind {
            handlers
                .iter()
                .position(|h| h == "GetAllItemsV1")
                .map(|sub| (e.idx, sub))
        } else {
            None
        }
    });
    assert!(
        get_all_v1_info.is_some(),
        "GetAllItemsV1 should be a delegate handler"
    );
    let (entry_idx, handler_idx) = get_all_v1_info.unwrap();
    println!(
        "  [MapCatalogApi] → ↓{}{} → GetAllItemsV1",
        entry_idx,
        (b'a' + handler_idx as u8) as char
    );

    step(
        &mut state,
        &Action::DownDispatch(entry_idx, handler_idx),
        &type_graph,
        &call_graph,
    );
    assert_eq!(state.current.method, "GetAllItemsV1");

    // Step 2: From GetAllItemsV1, navigate into EF query (ToListAsync, etc.)
    let query_idx = state
        .down
        .iter()
        .find(|e| {
            e.target.method.contains("ToListAsync")
                || e.target.method.contains("FirstOrDefault")
                || e.target.class.contains("CatalogContext")
        })
        .map(|e| e.idx);
    if let Some(idx) = query_idx {
        println!("  [GetAllItemsV1] → ↓{} → EF query", idx);
        step(&mut state, &Action::Down(idx), &type_graph, &call_graph);
        println!(
            "  Current: {}.{}",
            state.current.class, state.current.method
        );

        // Try one more deep step
        let deep_idx = state
            .down
            .iter()
            .find(|e| !matches!(e.kind, tiny_pdg_cs::traverse::types::EdgeKind::External))
            .map(|e| e.idx);
        if let Some(didx) = deep_idx {
            println!(
                "  Deeper → ↓{} → {}",
                didx,
                state
                    .down
                    .iter()
                    .find(|e| e.idx == didx)
                    .map(|e| e.target.method.as_str())
                    .unwrap_or("?")
            );
            step(&mut state, &Action::Down(didx), &type_graph, &call_graph);
            println!(
                "  Current: {}.{}",
                state.current.class, state.current.method
            );
        }
    }

    // Step 3: Complete + advance queue
    step(
        &mut state,
        &Action::Complete(Judgment::Primary),
        &type_graph,
        &call_graph,
    );
    if !state.queue.is_empty() {
        state.next_in_queue(&type_graph, &call_graph);
        println!("  Queue → {}.{}", state.current.class, state.current.method);
    }

    // ═══ RESULTS ═══════════════════════════════════════════════
    println!("\n═══ Navigation Results ═══");
    println!("  History ({} steps):", state.history.len());
    for (i, h) in state.history.iter().enumerate() {
        println!(
            "    [{}] {}.{} ↦ {}",
            i, h.node.class, h.node.method, h.action
        );
    }
    println!("  Final:  {}.{}", state.current.class, state.current.method);
    println!("  Queue:  {}", state.queue.len());
    println!("  Judgments: {}", state.judgments.len());

    assert!(
        state.history.len() >= 2,
        "Should have traversed at least 2 steps, got {}",
        state.history.len()
    );
}
