/// Example: parse an OTLP trace file and print SDG stats
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-trace.json> [source-dir]", args[0]);
        std::process::exit(1);
    }

    let trace_path = &args[1];

    match tinylink::sdg::otel::parse_trace_file(trace_path) {
        Ok(graph) => {
            println!("=== Service Dependency Graph ===");
            println!(
                "Traces: {} ({} errors)",
                graph.total_traces, graph.error_traces
            );
            println!(
                "Services: {} | Edges: {}",
                graph.nodes.len(),
                graph.edges.len()
            );

            println!("\nNodes:");
            for (name, node) in &graph.nodes {
                println!(
                    "  {} (endpoints: {}, errors: {})",
                    name,
                    node.endpoints.len(),
                    node.error_count
                );
            }

            println!("\nEdges:");
            for edge in &graph.edges {
                println!(
                    "  {} {} → {} (calls: {}, errors: {}, {:.0}ms)",
                    edge.source_endpoint,
                    edge.source,
                    edge.target,
                    edge.call_count,
                    edge.error_count,
                    edge.avg_latency_ms
                );
            }

            // If source dir provided, build unified graph
            if args.len() > 2 {
                let source_dir = &args[2];
                match tinylink::link::builder::build_unified(trace_path, source_dir) {
                    Ok(unified) => {
                        println!("\n=== Unified Graph ===");
                        println!(
                            "Nodes: {} | Edges: {} | Services: {}",
                            unified.nodes.len(),
                            unified.edges.len(),
                            unified.services.len()
                        );
                    }
                    Err(e) => eprintln!("Failed to build unified graph: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
