//! MCP server exposing trace→code linking to an AI agent.
//!
//! Requires a prebuilt index (`tinylink index ...`). See [`tools`] for the
//! rationale behind separating trace data from indexed code.

pub mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters,
    model::ServerConfig, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde_json::Value;

use tools::{BlastRadiusArgs, ExplainErrorArgs, FindMethodsArgs, RcaContext, ServicesArgs};

#[derive(Debug)]
pub struct TraceLinkServer {
    tool_router: ToolRouter<TraceLinkServer>,
    ctx: Arc<RcaContext>,
}

impl Default for TraceLinkServer {
    fn default() -> Self {
        Self::new(PathBuf::from("tinylink.db")).expect("default server")
    }
}

#[tool_router(router = tool_router)]
impl TraceLinkServer {
    /// Open a prebuilt index. Fails with build instructions if it is missing.
    pub fn new(index_path: PathBuf) -> anyhow::Result<Self> {
        let ctx = tools::open_context(index_path.as_path())?;
        Ok(Self {
            tool_router: Self::tool_router(),
            ctx,
        })
    }

    #[tool(
        name = "explain_error",
        description = "Explain a failing request: given an OTLP trace (inline JSON or a file path), return each errored service hop mapped to the C# method, file and line that handled it, with a confidence score. This is the main tool for \"why did this request fail\".",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn explain_error(
        &self,
        args: Parameters<ExplainErrorArgs>,
    ) -> Result<String, McpError> {
        wrap(tools::explain_error(&self.ctx, args.0))
    }

    #[tool(
        name = "blast_radius",
        description = "Given a Class.Method (or a bare class name), report the HTTP endpoints that reach it, its callers and its callees. Use for \"what does changing this affect\" and to find the entry point behind a failing method.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn blast_radius(
        &self,
        args: Parameters<BlastRadiusArgs>,
    ) -> Result<String, McpError> {
        wrap(tools::blast_radius(&self.ctx, args.0))
    }

    #[tool(
        name = "find_methods",
        description = "Search the indexed methods by class, method name or file path. Use it to learn the real names in the codebase before calling other tools, since guessing names wastes round trips.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn find_methods(
        &self,
        args: Parameters<FindMethodsArgs>,
    ) -> Result<String, McpError> {
        wrap(tools::find_methods(&self.ctx, args.0))
    }

    #[tool(
        name = "list_indexed_services",
        description = "List every service in the index with its method count and HTTP endpoints. Start here: it is the cheapest way to learn which services exist and how they are addressed.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_indexed_services(
        &self,
        args: Parameters<ServicesArgs>,
    ) -> Result<String, McpError> {
        wrap(tools::list_indexed_services(&self.ctx, args.0))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TraceLinkServer {
    fn get_info(&self) -> ServerConfig {
        // ServerConfig is #[non_exhaustive]; build then mutate.
        let mut info = ServerConfig::default();
        info.instructions = Some(
            "Links OpenTelemetry traces to indexed C# code. Answers \"why did this request \
             fail\": map an errored service hop to the method, file and line that handled \
             it, with a confidence score per mapping. All tools are read-only. Start with \
             list_indexed_services or find_methods to learn real class, method and route \
             names, then use explain_error with the trace and blast_radius with the method. \
             Questions about code structure alone (call graphs, PDGs, patterns) belong to the \
             tiny-pdg-cs MCP server instead."
                .into(),
        );
        info.capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .build();
        info
    }
}

/// Tools return JSON strings so the result stays machine-readable, and a bad
/// trace or an unknown method surfaces as an MCP error rather than killing the
/// server mid-session.
fn wrap(result: Result<Value, String>) -> Result<String, McpError> {
    result
        .map(|value| value.to_string())
        .map_err(|message| McpError::invalid_params(message, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_registers_the_four_link_tools() {
        let router = TraceLinkServer::tool_router();
        let names: Vec<String> = router
            .list_all()
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "explain_error",
            "blast_radius",
            "find_methods",
            "list_indexed_services",
        ] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
        assert_eq!(names.len(), 4, "unexpected tool set: {names:?}");
    }

    #[test]
    fn every_tool_is_read_only_and_documented() {
        let router = TraceLinkServer::tool_router();
        for tool in router.list_all() {
            let annotations = tool
                .annotations
                .as_ref()
                .unwrap_or_else(|| panic!("{} has no annotations", tool.name));
            assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
            let description = tool.description.as_deref().unwrap_or("");
            assert!(description.len() > 40, "{} needs a description", tool.name);
            assert!(
                !tool.input_schema.is_empty(),
                "{} missing schema",
                tool.name
            );
        }
    }

    #[test]
    fn new_fails_when_the_index_is_missing() {
        let err = match TraceLinkServer::new(PathBuf::from("/nonexistent/tinylink.db")) {
            Ok(_) => panic!("must not start without an index"),
            Err(e) => e,
        };
        let message = format!("{:#}", err);
        assert!(
            message.contains("tinylink index"),
            "startup error should say how to build one: {message}"
        );
    }

    #[test]
    fn server_info_points_at_the_pdg_server_for_code_questions() {
        let root = std::env::temp_dir().join(format!("tinylink_mcp_info_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let svc = root.join("Svc.API");
        std::fs::create_dir_all(&svc).unwrap();
        std::fs::write(svc.join("A.cs"), "namespace S;\npublic class A { }\n").unwrap();

        let db = root.join("index.db");
        let built = crate::link::index::build(&[svc.display().to_string()]).unwrap();
        crate::link::index::write(&built, &db).unwrap();

        let server = TraceLinkServer::new(db).expect("server");
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some());
        let instructions = info.instructions.expect("instructions");
        assert!(instructions.contains("explain_error"));
        assert!(
            instructions.contains("tiny-pdg-cs"),
            "instructions should route code-only questions to the other server: {instructions}"
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
