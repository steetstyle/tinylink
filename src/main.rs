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
    /// Serve trace-to-code linking over MCP for AI agents
    Serve {
        /// Path to a prebuilt index (see `tinylink index`)
        #[arg(long, default_value = "tinylink.db")]
        index: String,
        /// Serve over Streamable HTTP instead of stdio
        #[arg(long)]
        http: bool,
        /// Bind address for --http (loopback by default; no authentication)
        #[arg(long, default_value = "127.0.0.1")]
        bind: std::net::IpAddr,
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Endpoint path for --http
        #[arg(long, default_value = "mcp")]
        path: String,
    },
    /// Ingest OTLP log payloads into a log store
    LogsIngest {
        /// OTLP payload file, or a directory with one payload per file
        source: String,
        /// Log store file to write (defaults to TINYLINK_LOG_STORE)
        #[arg(long)]
        store: Option<String>,
    },
    /// Search a log store
    LogsSearch {
        /// Restrict to one trace id
        #[arg(long)]
        trace_id: Option<String>,
        /// Restrict to one service, as the exporter named it
        #[arg(long)]
        service: Option<String>,
        /// Minimum severity: Trace, Debug, Info, Warn, Error, Fatal
        #[arg(long)]
        min_severity: Option<String>,
        /// Substring, matched against the body and attribute values
        #[arg(long)]
        contains: Option<String>,
        /// Search attribute values only, ignoring the body
        #[arg(long)]
        attributes_only: bool,
        /// Only records having an attribute with this key
        #[arg(long)]
        attribute_key: Option<String>,
        /// Inclusive lower bound, Unix nanoseconds
        #[arg(long)]
        since: Option<u64>,
        /// Exclusive upper bound, Unix nanoseconds
        #[arg(long)]
        until: Option<u64>,
        #[arg(long, default_value = "50")]
        limit: usize,
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Log store file to read (defaults to TINYLINK_LOG_STORE)
        #[arg(long)]
        store: Option<String>,
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
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
        Commands::Serve {
            index,
            http,
            bind,
            port,
            path,
        } => serve(&index, http, bind, port, path),
        Commands::LogsIngest { source, store } => logs_ingest(&source, store.as_deref()),
        Commands::LogsSearch {
            trace_id,
            service,
            min_severity,
            contains,
            attributes_only,
            attribute_key,
            since,
            until,
            limit,
            offset,
            store,
            format,
        } => match logs_search_args(
            trace_id,
            service,
            min_severity,
            contains,
            attributes_only,
            attribute_key,
            since,
            until,
            limit,
            offset,
        ) {
            Ok(query) => logs_search(query, store.as_deref(), &format),
            // A bad flag should say what is valid rather than printing a usage
            // block, so this is routed through the same error path as the rest.
            Err(e) => Err(anyhow::Error::msg(format!("{e:#}"))),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Run the trace-to-code MCP server over stdio.
#[cfg(feature = "mcp")]
/// Run the trace-to-code MCP server.
///
/// Stdio by default, which is what MCP clients launch. `--http` serves Streamable
/// HTTP instead, for shared or remote deployments.
#[cfg(feature = "mcp")]
fn serve(
    index: &str,
    http: bool,
    bind: std::net::IpAddr,
    port: u16,
    path: String,
) -> anyhow::Result<()> {
    if http {
        return serve_http(bind, port, path, index);
    }
    serve_stdio(index)
}

#[cfg(all(feature = "mcp", not(feature = "mcp-http")))]
fn serve_http(
    _bind: std::net::IpAddr,
    _port: u16,
    _path: String,
    _index: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("`serve --http` requires the `mcp-http` feature: cargo build --features mcp-http")
}

#[cfg(feature = "mcp-http")]
fn serve_http(bind: std::net::IpAddr, port: u16, path: String, index: &str) -> anyhow::Result<()> {
    let args = tinylink::mcp::http::HttpArgs { bind, port, path };
    tinylink::mcp::http::serve_http(args, std::path::Path::new(index))
}

#[cfg(feature = "mcp")]
fn serve_stdio(index: &str) -> anyhow::Result<()> {
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

/// The `mcp` feature is required for `serve`; keep the other commands usable
/// without it.
#[cfg(not(feature = "mcp"))]
fn serve(
    _index: &str,
    _http: bool,
    _bind: std::net::IpAddr,
    _port: u16,
    _path: String,
) -> anyhow::Result<()> {
    anyhow::bail!("`serve` requires the `mcp` feature: cargo build --features mcp")
}

// ───────────────────────── logs ─────────────────────────

/// Where a log store lives when `--store` is absent.
///
/// The MCP tools resolve the same way, so a CLI ingest and a server query can be
/// pointed at one file.
fn resolve_log_store(store: Option<&str>) -> std::path::PathBuf {
    match store {
        Some(path) => std::path::PathBuf::from(path),
        None => match std::env::var("TINYLINK_LOG_STORE") {
            Ok(path) if !path.is_empty() => std::path::PathBuf::from(path),
            _ => std::path::PathBuf::from(DEFAULT_LOG_STORE),
        },
    }
}

/// Where `logs-ingest` and `logs-search` write and read when nothing says otherwise.
///
/// Kept next to `resolve_log_store` so the warning below and the path it names come
/// from one place.
const DEFAULT_LOG_STORE: &str = "tinylink.logs.db";

/// Say what a server would read, when this write is not where a server reads.
///
/// The two defaults are genuinely different and neither is wrong on its own: `ingest`
/// has no index to derive from, so it writes beside the working directory, while
/// `serve` has one and so writes beside it. Used together with no `--store` and no
/// `TINYLINK_LOG_STORE`, they never meet -- measured, after ingesting 29,347 records
/// into `./tinylink.logs.db` and searching a store derived from the index, which
/// answered that the log store was empty.
fn warn_about_a_server_that_reads_elsewhere(store: Option<&str>, written: &std::path::Path) {
    let named_by_user =
        store.is_some() || std::env::var("TINYLINK_LOG_STORE").is_ok_and(|v| !v.is_empty());
    let default = std::path::Path::new(DEFAULT_LOG_STORE);

    if written == default && !named_by_user {
        eprintln!(
            "note: wrote {}. A server started with `tinylink serve --index <path>` reads \
             <path>.logs.db instead -- beside its index, not beside the working \
             directory -- so it will report an empty store. Name one place and set it \
             for both: export TINYLINK_LOG_STORE={}",
            written.display(),
            written.display()
        );
    } else if !named_by_user {
        eprintln!(
            "note: wrote {}. A server reads the store beside its index unless \
             TINYLINK_LOG_STORE says otherwise, so check that `tinylink serve` was \
             started with --index {} (or that both sides set TINYLINK_LOG_STORE).",
            written.display(),
            written.with_extension("db").display()
        );
    }
}

fn logs_ingest(source: &str, store: Option<&str>) -> anyhow::Result<()> {
    use tinylink::sdg::log_store::{LogStore, LogStorePath};

    // `store` the argument is shadowed by the LogStore below, and the warning needs to
    // know whether the caller named a path.
    let asked_for = store;
    let path = resolve_log_store(store);
    let mut store = LogStore::open(LogStorePath::File(path.clone()))?;
    let report = store.insert_path(std::path::Path::new(source))?;

    warn_about_a_server_that_reads_elsewhere(asked_for, &path);
    if report.duplicates > 0 {
        eprintln!(
            "note: {} record(s) were already in the store and were not added again \
             (records_inserted: {}).",
            report.duplicates, report.inserted
        );
    }
    println!(
        "Ingested {} record(s) from {} payload(s) into {}",
        report.inserted,
        report.payloads,
        path.display()
    );
    println!("  distinct traces:  {}", report.traces);
    println!("  distinct services: {}", report.services);
    if !report.skipped.is_empty() {
        // Never silent: a payload with no service.name makes every later service
        // filter return nothing, and that looks like "no logs" rather than
        // "logs we cannot attribute".
        println!("  problems:");
        for problem in &report.skipped {
            println!("    - {problem}");
        }
    }
    Ok(())
}

fn logs_search(
    query: tinylink::sdg::log_store::LogQuery,
    store: Option<&str>,
    format: &str,
) -> anyhow::Result<()> {
    use tinylink::sdg::log_store::{LogStore, LogStorePath};

    let path = resolve_log_store(store);
    let store = LogStore::open(LogStorePath::File(path.clone()))?;
    let result = tinylink::sdg::log_store::search_logs_json(&store, &query)?;

    if format == "json" {
        println!("{result}");
        return Ok(());
    }

    let logs = result["logs"].as_array().cloned().unwrap_or_default();
    if logs.is_empty() {
        // Say what was searched, so "no match" is not read as "no logs exist".
        println!(
            "No matching logs. Store holds {} record(s).",
            result["store_records"]
        );
        return Ok(());
    }

    for entry in &logs {
        println!(
            "[{}] {} {} — {}",
            entry["severity"].as_str().unwrap_or("?"),
            entry["time_unix_nano"].as_str().unwrap_or("?"),
            entry["service"].as_str().unwrap_or("?"),
            entry["body"].as_str().unwrap_or("")
        );
        if let Some(trace) = entry["trace_id"].as_str() {
            println!("    trace {trace}");
        }
        let attributes = entry["attributes"].as_object().cloned().unwrap_or_default();
        for (key, value) in attributes {
            // Printed unquoted: the rendered message is what a human reads, and it
            // only exists once the template's placeholders are filled from these
            // attributes. JSON quoting would break copy-paste of an exception.
            let rendered = match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            println!("    {key} = {rendered}");
        }
    }
    println!(
        "\nShowing {} of {} match(es) in a store of {} record(s).",
        result["count"], result["total_matched"], result["store_records"]
    );
    if result["has_more"] == serde_json::json!(true) {
        println!("More results exist; raise --limit or pass --offset.");
    }
    if result["excluded_untimed"].as_u64().unwrap_or(0) > 0 {
        println!(
            "{} record(s) carry no timestamp and cannot be matched by a time window.",
            result["excluded_untimed"]
        );
    }
    Ok(())
}

/// Build a query from CLI flags.
///
/// Kept separate from `main` so the argument list is not another long positional
/// run at the call site, and so a missing flag is visibly `None` rather than a
/// default that reads as a filter.
#[allow(clippy::too_many_arguments)]
fn logs_search_args(
    trace_id: Option<String>,
    service: Option<String>,
    min_severity: Option<String>,
    contains: Option<String>,
    attributes_only: bool,
    attribute_key: Option<String>,
    since: Option<u64>,
    until: Option<u64>,
    limit: usize,
    offset: usize,
) -> anyhow::Result<tinylink::sdg::log_store::LogQuery> {
    use tinylink::sdg::log_store::LogQuery;

    Ok(LogQuery {
        trace_id,
        service,
        min_severity: min_severity
            .as_deref()
            .map(LogQuery::parse_severity)
            .transpose()?,
        contains,
        attributes_only,
        attribute_key,
        since_unix_nano: since,
        until_unix_nano: until,
        only_traceable: false,
        limit: LogQuery::bounded_limit(Some(limit))?,
        offset,
    })
}
