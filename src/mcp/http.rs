//! Serving the trace-to-code MCP server over Streamable HTTP.
//!
//! # Why
//!
//! The MCP spec deprecated the standalone SSE transport and replaced it with
//! Streamable HTTP, and the Rust SDK ships no SSE server, so the supported pair
//! is:
//!
//! - **stdio** (`tinylink serve --index ...`) — a client launches the binary.
//!   No port, no listening socket. Best for a local agent.
//! - **Streamable HTTP** (`tinylink serve --http`) — one endpoint that several
//!   clients, or another process, can reach. Needed in a container or when the
//!   index is built once and shared.
//!
//! Both expose the same four tools; only the framing differs.
//!
//! # Security
//!
//! Binds loopback by default and has **no authentication** — the MCP
//! authorization spec expects an OAuth flow, which is not implemented here. Do
//! not bind it to a public interface without putting an authenticating proxy in
//! front.

use std::net::{IpAddr, SocketAddr};

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;

use crate::mcp::TraceLinkServer;

/// Default mount path.
pub const DEFAULT_PATH: &str = "/mcp";

#[derive(Debug, Clone, clap::Parser)]
pub struct HttpArgs {
    /// Address to bind. Loopback only by default: no authentication here.
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: IpAddr,
    #[arg(long, default_value = "8080")]
    pub port: u16,
    /// Path to mount the MCP endpoint at
    #[arg(long, default_value = DEFAULT_PATH)]
    pub path: String,
}

/// Serve over Streamable HTTP until the process is stopped.
pub fn serve_http(args: HttpArgs, index_path: &std::path::Path) -> anyhow::Result<()> {
    let addr = SocketAddr::new(args.bind, args.port);

    if !args.bind.is_loopback() {
        tracing::warn!(
            "binding {addr} on a non-loopback address without authentication; \
             put an authenticating proxy in front before exposing it"
        );
    }

    // Load the index before binding: failing here means a clear error rather than
    // an apparently healthy server that answers every tool call wrongly.
    let ctx = crate::mcp::tools::open_context(index_path)?;
    tracing::info!(
        methods = ctx.index.code.methods.len(),
        routes = ctx.index.routes.entries.len(),
        "index loaded"
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run(args, addr, ctx))
}

async fn run(
    args: HttpArgs,
    addr: SocketAddr,
    ctx: std::sync::Arc<crate::mcp::tools::RcaContext>,
) -> anyhow::Result<()> {
    let path = if args.path.starts_with('/') {
        args.path.clone()
    } else {
        format!("/{}", args.path)
    };

    // The index is captured once and shared: each HTTP session gets its own
    // server instance but they all read the same immutable index.
    // Clone into the factory so the Arc is still available for the log line
    // below; the index is immutable, so sharing it across sessions is safe.
    let shared = std::sync::Arc::clone(&ctx);
    let service: StreamableHttpService<TraceLinkServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(TraceLinkServer::with_context(std::sync::Arc::clone(
                    &shared,
                )))
            },
            Default::default(),
            Default::default(),
        );

    let router = axum::Router::new().nest_service(&path, service);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("cannot bind {addr}: {e}"))?;
    let local = listener.local_addr().unwrap_or(addr);
    // stderr, not stdout: stdout belongs to the stdio transport.
    eprintln!(
        "tinylink MCP listening on http://{local}{path} (streamable HTTP), index {}",
        ctx.path.display()
    );

    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("server stopped: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    #[test]
    fn http_args_default_to_loopback() {
        let args = HttpArgs::parse_from(["tinylink"]);
        assert!(
            args.bind.is_loopback(),
            "must not listen publicly by default"
        );
        assert_eq!(args.port, 8080);
        assert_eq!(args.path, DEFAULT_PATH);
    }

    #[test]
    fn http_args_parse_overrides() {
        let args =
            HttpArgs::parse_from(["x", "--bind", "0.0.0.0", "--port", "9111", "--path", "rpc"]);
        assert_eq!(args.bind.to_string(), "0.0.0.0");
        assert_eq!(args.port, 9111);
        assert_eq!(args.path, "rpc");
    }
}
