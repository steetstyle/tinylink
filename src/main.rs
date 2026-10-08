use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tinylink", about = "SDG + PDG unification bridge")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build service dependency graph from OTLP trace data
    Sdg {
        #[arg(help = "Path to OTLP JSON trace file")]
        trace: String,
        #[arg(long, help = "Output format: json, dot")]
        format: Option<String>,
        #[arg(long, help = "Show service metrics (error count, latency)")]
        stats: bool,
    },
    /// Analyze C# source code and extract code graph via tiny-pdg-cs
    Code {
        #[arg(help = "Path to C# project directory")]
        source: String,
        #[arg(long, help = "Filter by service name")]
        service: Option<String>,
        #[arg(long, help = "Output format: json, dot")]
        format: Option<String>,
    },
    /// Unify SDG and code graph into a single graph
    Link {
        #[arg(help = "Path to OTLP JSON trace file")]
        trace: String,
        #[arg(help = "Path(s) to C# project directories (comma-separated)")]
        sources: String,
        #[arg(long, help = "Output format: json, dot")]
        format: Option<String>,
    },
    /// Trace a single request through services and code
    Tracepath {
        #[arg(help = "Path to OTLP JSON trace file")]
        trace: String,
        #[arg(help = "Path(s) to C# project directories (comma-separated)")]
        sources: String,
        #[arg(long, help = "Trace ID to follow")]
        trace_id: Option<String>,
    },
    /// Query error edges and their code locations
    Query {
        #[arg(help = "Path to OTLP JSON trace file")]
        trace: String,
        #[arg(help = "Path(s) to C# project directories (comma-separated)")]
        sources: String,
        #[arg(long, help = "Filter by service name")]
        service: Option<String>,
        #[arg(long, help = "Also show PDG for errored method")]
        pdg: bool,
    },
    /// Query Prometheus for service graph metrics (requires `online` feature)
    #[cfg(feature = "online")]
    Prometheus {
        #[arg(help = "Prometheus URL (e.g. http://localhost:9090)")]
        url: String,
        #[arg(long, default_value = "5m", help = "Time range (e.g. 5m, 1h)")]
        since: String,
        #[arg(long, help = "Output format: json, dot")]
        format: Option<String>,
    },
    /// Build a reusable SQLite index of code, routes and call edges
    Index {
        #[arg(help = "Path(s) to C# project directories (comma-separated)")]
        sources: String,
        #[arg(long, short, help = "Output path for the index file")]
        out: String,
        /// Rebuild even when the sources are unchanged since the last build
        #[arg(long)]
        force: bool,
        /// Print a summary without writing the file
        #[arg(long)]
        dry_run: bool,
    },
    /// Serve trace-to-code linking over MCP (stdio) for AI agents
    Serve {
        /// Path to a prebuilt index (see `tinylink index`)
        #[arg(long, default_value = "tinylink.db")]
        index: String,
    },
    /// Generate PDG (Program Dependence Graph) DOT for a method
    Pdg {
        #[arg(help = "Path to C# project directory")]
        source: String,
        #[arg(long, help = "Class name (optional: all methods if empty)")]
        class: Option<String>,
        #[arg(long, help = "Method name (optional: all methods if empty)")]
        method: Option<String>,
        #[arg(long, help = "Output file path (prints to stdout if omitted)")]
        output: Option<String>,
    },
    /// Interactive cross-service exploration
    Explore {
        #[arg(help = "Path to OTLP JSON trace file (or Prometheus URL with --from)")]
        source: String,
        #[arg(help = "Path(s) to C# project directories (comma-separated)")]
        sources: String,
        #[arg(long, help = "Source type: otel (default) or prometheus")]
        from: Option<String>,
        #[arg(long, help = "Generate PDG DOT for each errored method")]
        pdg: bool,
    },
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Sdg {
            trace,
            format,
            stats,
        } => tinylink::cli::commands::handle_sdg(&trace, format.as_deref(), stats),
        Commands::Code {
            source,
            service,
            format,
        } => tinylink::cli::commands::handle_code(&source, service.as_deref(), format.as_deref()),
        Commands::Link {
            trace,
            sources,
            format,
        } => tinylink::cli::commands::handle_link(&trace, &sources, format.as_deref()),
        Commands::Tracepath {
            trace,
            sources,
            trace_id,
        } => tinylink::cli::commands::handle_tracepath(&trace, &sources, trace_id.as_deref()),
        Commands::Query {
            trace,
            sources,
            service,
            pdg,
        } => tinylink::cli::commands::handle_query(&trace, &sources, service.as_deref(), pdg),
        #[cfg(feature = "online")]
        Commands::Prometheus { url, since, format } => {
            tinylink::cli::commands::handle_prometheus(&url, &since, format.as_deref())
        }
        Commands::Pdg {
            source,
            class,
            method,
            output,
        } => tinylink::cli::commands::handle_pdg(
            &source,
            class.as_deref().unwrap_or(""),
            method.as_deref().unwrap_or(""),
            output.as_deref(),
        ),
        Commands::Explore {
            source,
            sources,
            from,
            pdg,
        } => tinylink::cli::commands::handle_explore(&source, &sources, from.as_deref(), pdg),
        Commands::Index {
            sources,
            out,
            force,
            dry_run,
        } => tinylink::cli::commands::handle_index(&sources, &out, force, dry_run),
        Commands::Serve { index } => serve(&index),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Run the trace-to-code MCP server over stdio.
#[cfg(feature = "mcp")]
fn serve(index: &str) -> anyhow::Result<()> {
    use rmcp::transport::stdio;
    use rmcp::ServiceExt;

    let server = tinylink::mcp::TraceLinkServer::new(std::path::PathBuf::from(index))?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let service = server.serve(stdio()).await?;
        service.waiting().await?;
        Ok::<(), anyhow::Error>(())
    })
}

#[cfg(not(feature = "mcp"))]
fn serve(_index: &str) -> anyhow::Result<()> {
    anyhow::bail!("`serve` requires the `mcp` feature: cargo build --features mcp")
}
