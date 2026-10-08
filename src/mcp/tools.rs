//! MCP server bridging OpenTelemetry traces to indexed C# code.
//!
//! Answers "this request failed — which line of which service caused it". The
//! code side is read from a prebuilt [`crate::link::index`] so a tool call costs
//! a SQL query rather than a full reparse; the runtime side comes from the
//! caller, since traces are per-incident data and deliberately not indexed.
//!
//! Code-only questions ("who calls this", "what does this PDG look like") belong
//! to the tiny-pdg-cs MCP server. This one exists for the join between the two.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::link::builder;
use crate::link::index::{self, Index};
use crate::link::mapper;
use crate::sdg::log_store::{LogQuery, LogStore};
use crate::sdg::otel;

/// An opened index, held for the server's lifetime.
#[derive(Debug)]
pub struct RcaContext {
    pub index: Index,
    pub path: std::path::PathBuf,
}

/// Open an index file, with an error message that says how to build one.
pub fn open_context(path: &Path) -> Result<Arc<RcaContext>> {
    let conn = index::open(path)?;
    index::check_schema(&conn)?;
    let loaded = index::load(path)?;
    Ok(Arc::new(RcaContext {
        index: loaded,
        path: path.to_path_buf(),
    }))
}

// ───────────────────────── explain_error ─────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExplainErrorArgs {
    /// OTLP trace JSON: either an inline document or a path to a `.json` file.
    pub trace: String,
    /// Report only edges touching this service.
    pub service: Option<String>,
    /// Cap the number of edges returned, most errored first.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Where a failing request ended up in the code.
pub fn explain_error(ctx: &RcaContext, args: ExplainErrorArgs) -> Result<Value, String> {
    let sdg = load_trace(&args.trace)?;

    let errors: Vec<_> = sdg
        .edges
        .iter()
        .filter(|e| e.error_count > 0)
        .filter(|e| match args.service.as_deref() {
            None => true,
            Some(s) => e.source == s || e.target == s,
        })
        .collect();

    if errors.is_empty() {
        return Ok(json!({
            "error_edges": [],
            "message": "no errored edges in this trace",
            "total_edges": sdg.edges.len(),
            "total_traces": sdg.total_traces,
        }));
    }

    let unified = mapper::build_unified_from_sdg(&sdg, &ctx.index.code, &ctx.index.routes);

    let mut reported: Vec<Value> = Vec::new();
    // Most errored first: the likely culprit is where errors concentrate.
    let mut ranked: Vec<_> = errors.iter().collect();
    ranked.sort_by(|a, b| b.error_count.cmp(&a.error_count));

    let limit = args.limit.unwrap_or(20).max(1);
    for edge in ranked.into_iter().take(limit) {
        if let Some(ue) = unified
            .edges
            .iter()
            .find(|e| e.source_service == edge.source && e.target_service == edge.target)
        {
            reported.push(json!({
                "from": {
                    "service": ue.source_service,
                    "method": ue.source_method,
                    "confidence": ue.source_confidence,
                },
                "to": {
                    "service": ue.target_service,
                    "method": ue.target_method,
                    "confidence": ue.target_confidence,
                    "file": ue.target_file,
                    "line": ue.target_line,
                },
                "error_count": edge.error_count,
                "call_count": edge.call_count,
                "avg_latency_ms": edge.avg_latency_ms,
                "trace_ids": edge.trace_ids,
                "endpoint": {
                    "source": edge.source_endpoint,
                    "target": edge.target_endpoint,
                },
            }));
        } else {
            // The edge exists but did not survive unification; report it rather
            // than silently dropping a failing hop.
            reported.push(json!({
                "from": {"service": edge.source, "method": Value::Null},
                "to": {"service": edge.target, "method": Value::Null},
                "error_count": edge.error_count,
                "call_count": edge.call_count,
                "avg_latency_ms": edge.avg_latency_ms,
                "endpoint": {
                    "source": edge.source_endpoint,
                    "target": edge.target_endpoint,
                },
                "note": "no code mapping resolved for this edge",
            }));
        }
    }

    Ok(json!({
        "error_edges": reported,
        "total_edges": sdg.edges.len(),
        "total_traces": sdg.total_traces,
        "error_traces": sdg.error_traces,
        "index": ctx.path.display().to_string(),
    }))
}

/// Read a trace from an inline JSON document or a file path.
fn load_trace(input: &str) -> Result<crate::sdg::types::ServiceGraph, String> {
    let trimmed = input.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        otel::parse_trace_json(trimmed)
            .map_err(|e| format!("failed to parse inline trace JSON: {:#}", e))
    } else {
        otel::parse_trace_file(input)
            .map_err(|e| format!("failed to read trace file {input}: {:#}", e))
    }
}

// ───────────────────────── blast_radius ─────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BlastRadiusArgs {
    /// `Class.Method`, or `Class` to report every method of that class.
    pub target: String,
    /// Restrict to this service when the target is ambiguous.
    pub service: Option<String>,
}

/// Which endpoints and code paths reach a method.
pub fn blast_radius(ctx: &RcaContext, args: BlastRadiusArgs) -> Result<Value, String> {
    let code_graph = &ctx.index.code;

    let matches: Vec<_> = if let Some((class, method)) = args.target.split_once('.') {
        code_graph
            .methods
            .iter()
            .filter(|m| m.class == class && m.method == method)
            .collect()
    } else {
        code_graph
            .methods
            .iter()
            .filter(|m| m.class == args.target)
            .collect()
    };

    let scoped: Vec<_> = match args.service.as_deref() {
        None => matches,
        Some(s) => matches
            .into_iter()
            .filter(|m| m.service.as_deref() == Some(s))
            .collect(),
    };

    if scoped.is_empty() {
        return Err(format!(
            "no method matching '{}'{}; the index holds {} methods. Try list_indexed_services.",
            args.target,
            args.service
                .as_ref()
                .map(|s| format!(" in service {s}"))
                .unwrap_or_default(),
            code_graph.methods.len()
        ));
    }

    let mut results = Vec::new();
    for m in scoped {
        let full = format!("{}.{}", m.class, m.method);

        // HTTP entry points: the routes whose handler is this method.
        let routes: Vec<Value> = ctx
            .index
            .routes
            .entries
            .iter()
            .filter(|r| r.class == m.class && r.method == m.method)
            .map(|r| {
                json!({
                    "method": r.http_method,
                    "pattern": r.route_pattern,
                    "service": r.service,
                })
            })
            .collect();

        // Callers: who could have reached this method from inside the code.
        let callers: Vec<Value> = code_graph
            .calls
            .iter()
            .filter(|c| c.target_class == m.class && c.target_method == m.method)
            .map(|c| {
                json!({
                    "class": c.source_class,
                    "method": c.source_method,
                    "line": c.line,
                })
            })
            .collect();

        let callees: Vec<Value> = code_graph
            .calls
            .iter()
            .filter(|c| c.source_class == m.class && c.source_method == m.method)
            .map(|c| {
                json!({
                    "class": c.target_class,
                    "method": c.target_method,
                    "line": c.line,
                })
            })
            .collect();

        results.push(json!({
            "method": full,
            "service": m.service,
            "file": m.file,
            "line_start": m.line_start,
            "line_end": m.line_end,
            "signature": m.signature,
            "routes": routes,
            "callers": callers,
            "callees": callees,
        }));
    }

    Ok(json!({
        "results": results,
        "index": ctx.path.display().to_string(),
    }))
}

// ───────────────────────── services & lookup ─────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ServicesArgs {}

/// What is in this index, and which endpoints each service exposes.
pub fn list_indexed_services(ctx: &RcaContext, _args: ServicesArgs) -> Result<Value, String> {
    let mut by_service: std::collections::BTreeMap<String, (usize, Vec<Value>)> =
        std::collections::BTreeMap::new();

    for m in &ctx.index.code.methods {
        let service = m.service.clone().unwrap_or_else(|| "(unknown)".into());
        by_service.entry(service).or_default().0 += 1;
    }
    for r in &ctx.index.routes.entries {
        let service = r.service.clone().unwrap_or_else(|| "(unknown)".into());
        by_service.entry(service).or_default().1.push(json!({
            "method": r.http_method,
            "pattern": r.route_pattern,
            "handler": format!("{}.{}", r.class, r.method),
        }));
    }

    let services: Vec<Value> = by_service
        .into_iter()
        .map(|(name, (methods, routes))| {
            json!({
                "service": name,
                "methods": methods,
                "routes": routes.len(),
                "endpoints": routes,
            })
        })
        .collect();

    Ok(json!({
        "services": services,
        "total_methods": ctx.index.code.methods.len(),
        "total_routes": ctx.index.routes.entries.len(),
        "content_hash": ctx.index.content_hash,
        "inline_lambda_routes": ctx.index.inline_lambda_routes,
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindMethodsArgs {
    /// Substring to look for in class, method or file. Omit to list everything.
    pub query: Option<String>,
    /// Restrict to one service.
    pub service: Option<String>,
    /// Cap the number of rows.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Locate methods in the index without knowing their exact name.
pub fn find_methods(ctx: &RcaContext, args: FindMethodsArgs) -> Result<Value, String> {
    let needle = args.query.as_ref().map(|q| q.to_lowercase());
    let limit = args.limit.unwrap_or(50).max(1);

    let matches: Vec<Value> = ctx
        .index
        .code
        .methods
        .iter()
        .filter(|m| match args.service.as_deref() {
            None => true,
            Some(s) => m.service.as_deref() == Some(s),
        })
        .filter(|m| match needle.as_ref() {
            None => true,
            Some(n) => {
                m.class.to_lowercase().contains(n)
                    || m.method.to_lowercase().contains(n)
                    || m.file.to_lowercase().contains(n)
            }
        })
        .take(limit)
        .map(|m| {
            json!({
                "class": m.class,
                "method": m.method,
                "service": m.service,
                "file": m.file,
                "line_start": m.line_start,
            })
        })
        .collect();

    Ok(json!({
        "methods": matches,
        "count": matches.len(),
        "index": ctx.path.display().to_string(),
    }))
}

// ───────────────────────── builder re-export ─────────────────────────

/// Parse a comma-separated `--sources` list.
pub fn split_sources(sources: &str) -> Result<Vec<String>> {
    builder::split_sources(sources).context("invalid sources list")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(name: &str) -> (Arc<RcaContext>, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("tinylink_mcp_{}", name));
        let _ = std::fs::remove_dir_all(&root);
        let agency = root.join("Agency.API");
        std::fs::create_dir_all(&agency).unwrap();
        std::fs::write(
            agency.join("Handler.cs"),
            r#"
namespace Agency.API;
public class GetMembersEndpoint
{
    public static IResult Handler(HttpContext ctx) { return Ok(); }
    static IResult Ok() => null;
}
"#,
        )
        .unwrap();
        std::fs::write(
            agency.join("Ext.cs"),
            "var api = app.MapGroup(\"\");\napi.MapGet(\"/api/agency/{agencyId:guid}/member\", GetMembersEndpoint.Handler);\n",
        )
        .unwrap();

        let db = root.join("index.db");
        let built = index::build(&[agency.display().to_string()]).unwrap();
        index::write(&built, &db).unwrap();
        let ctx = open_context(&db).expect("open index");
        (ctx, root)
    }

    const FAILING_TRACE: &str = r#"{"resourceSpans":[
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Dashboard.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"t-err","spanId":"s1","parentSpanId":"","name":"GET","kind":3,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.target","value":{"stringValue":"/api/agency/9f1c/member"}}],
    "startTimeUnixNano":0,"endTimeUnixNano":100000000}]}]},
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Agency.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"t-err","spanId":"s2","parentSpanId":"s1","name":"GET","kind":2,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.route","value":{"stringValue":"/api/agency/{agencyId}/member"}},
                  {"key":"http.status_code","value":{"intValue":500}}],
    "startTimeUnixNano":5000000,"endTimeUnixNano":90000000}]}]}
]}"#;

    #[test]
    fn explain_error_maps_a_500_to_the_handler_file_and_line() {
        let (ctx, root) = context("explain");
        let out = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: FAILING_TRACE.to_string(),
                service: None,
                limit: None,
            },
        )
        .expect("explain_error");

        let edges = out["error_edges"].as_array().expect("error_edges");
        assert_eq!(edges.len(), 1, "{out}");
        let e = &edges[0];
        assert_eq!(e["error_count"], 1);
        assert_eq!(
            e["to"]["method"], "GetMembersEndpoint.Handler",
            "must resolve to the endpoint handler: {e}"
        );
        assert_eq!(e["to"]["confidence"], 0.95);
        let file = e["to"]["file"].as_str().expect("file must be set");
        assert!(file.ends_with("Handler.cs"), "got {file}");
        assert!(e["to"]["line"].as_u64().unwrap() > 0);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn explain_error_accepts_a_trace_file_path() {
        let (ctx, root) = context("explain_file");
        let trace_file = root.join("trace.json");
        std::fs::write(&trace_file, FAILING_TRACE).unwrap();

        let out = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: trace_file.display().to_string(),
                service: None,
                limit: None,
            },
        )
        .expect("explain_error from file");
        assert_eq!(out["error_edges"].as_array().unwrap().len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn explain_error_reports_cleanly_when_nothing_failed() {
        let (ctx, root) = context("explain_clean");
        let clean = r#"{"resourceSpans":[
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Agency.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"t-ok","spanId":"a","parentSpanId":"","name":"GET","kind":2,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}}],
    "startTimeUnixNano":0,"endTimeUnixNano":1000000}]}]}
]}"#;
        let out = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: clean.to_string(),
                service: None,
                limit: None,
            },
        )
        .expect("explain_error");
        assert!(out["error_edges"].as_array().unwrap().is_empty(), "{out}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn explain_error_rejects_malformed_trace() {
        let (ctx, root) = context("explain_bad");
        let err = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: "{not json".to_string(),
                service: None,
                limit: None,
            },
        )
        .expect_err("malformed input must error");
        assert!(err.contains("parse"), "unhelpful error: {err}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn service_filter_narrows_the_error_edges() {
        let (ctx, root) = context("explain_filter");
        let hit = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: FAILING_TRACE.to_string(),
                service: Some("Agency.API".into()),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(hit["error_edges"].as_array().unwrap().len(), 1);

        let miss = explain_error(
            &ctx,
            ExplainErrorArgs {
                trace: FAILING_TRACE.to_string(),
                service: Some("Unrelated.Service".into()),
                limit: None,
            },
        )
        .unwrap();
        assert!(
            miss["error_edges"].as_array().unwrap().is_empty(),
            "a non-matching service must yield no edges"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blast_radius_reports_the_endpoint_and_callers() {
        let (ctx, root) = context("blast");
        let out = blast_radius(
            &ctx,
            BlastRadiusArgs {
                target: "GetMembersEndpoint.Handler".into(),
                service: None,
            },
        )
        .expect("blast_radius");

        let results = out["results"].as_array().expect("results");
        assert_eq!(results.len(), 1, "{out}");
        let r = &results[0];
        // The HTTP entry point is what "blast radius" means for an endpoint.
        let routes = r["routes"].as_array().unwrap();
        assert_eq!(routes.len(), 1, "{r}");
        assert_eq!(routes[0]["pattern"], "/api/agency/{agencyId}/member");
        assert_eq!(routes[0]["method"], "GET");
        // Handler calls Ok, so it has a callee.
        assert!(!r["callees"].as_array().unwrap().is_empty(), "{r}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blast_radius_accepts_a_bare_class_name() {
        let (ctx, root) = context("blast_class");
        let out = blast_radius(
            &ctx,
            BlastRadiusArgs {
                target: "GetMembersEndpoint".into(),
                service: None,
            },
        )
        .expect("blast_radius");
        let results = out["results"].as_array().unwrap();
        assert!(
            results.len() >= 2,
            "a class target lists its methods: {out}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blast_radius_errors_with_a_hint_for_an_unknown_target() {
        let (ctx, root) = context("blast_missing");
        let err = blast_radius(
            &ctx,
            BlastRadiusArgs {
                target: "NoSuchThing.method".into(),
                service: None,
            },
        )
        .expect_err("unknown target must error");
        assert!(
            err.contains("list_indexed_services"),
            "error should point at a discovery tool: {err}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_indexed_services_reports_counts_and_endpoints() {
        let (ctx, root) = context("services");
        let out = list_indexed_services(&ctx, ServicesArgs {}).expect("list services");
        let services = out["services"].as_array().expect("services");
        assert_eq!(services.len(), 1, "{out}");
        assert_eq!(services[0]["service"], "Agency.API");
        assert_eq!(services[0]["routes"], 1);
        assert!(services[0]["methods"].as_u64().unwrap() >= 2);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_methods_searches_class_method_and_file() {
        let (ctx, root) = context("find");
        let by_class = find_methods(
            &ctx,
            FindMethodsArgs {
                query: Some("GetMembers".into()),
                service: None,
                limit: None,
            },
        )
        .unwrap();
        assert!(by_class["count"].as_u64().unwrap() >= 1, "{by_class}");

        let by_file = find_methods(
            &ctx,
            FindMethodsArgs {
                query: Some("Ext.cs".into()),
                service: None,
                limit: None,
            },
        )
        .unwrap();
        // Ext.cs declares no methods, so a file search legitimately finds nothing;
        // what matters is that the search runs and answers.
        assert!(by_file["methods"].is_array(), "{by_file}");

        let capped = find_methods(
            &ctx,
            FindMethodsArgs {
                query: None,
                service: None,
                limit: Some(1),
            },
        )
        .unwrap();
        assert_eq!(capped["count"], 1, "limit must be honoured: {capped}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn opening_a_missing_index_says_how_to_build_it() {
        let err = match open_context(Path::new("/nonexistent/tinylink/index.db")) {
            Ok(_) => panic!("a missing index must fail to open"),
            Err(e) => e,
        };
        let message = format!("{:#}", err);
        assert!(message.contains("tinylink index"), "{message}");
    }
}

// ───────────────────────── logs ─────────────────────────

/// Where a log store lives when the caller does not say.
///
/// Environment first so a deployment can point it at a mounted volume, then the
/// code index path with a different extension, so the store and the index stay
/// together and a relative invocation still finds the logs it just ingested. The
/// CLI resolves its default the same way.
pub fn default_log_store_path(index_path: &Path) -> PathBuf {
    if let Ok(from_env) = std::env::var("TINYLINK_LOG_STORE") {
        if !from_env.is_empty() {
            return PathBuf::from(from_env);
        }
    }
    let mut path = index_path.to_path_buf();
    path.set_extension("logs.db");
    path
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IngestLogsArgs {
    /// An OTLP payload file, or a directory of them (one payload per file).
    ///
    /// A directory is walked one level deep, skipping the log store's own files.
    /// Several protobuf exports concatenated into a single stream cannot be split
    /// reliably — OTLP carries no total length — so one payload per file is
    /// required rather than guessed at.
    pub source: String,
}

/// Read OTLP log payloads into the store.
///
/// Kept separate from querying because ingest is the step that needs explaining:
/// a payload with no `service.name`, or a file that is not a payload at all, both
/// have to be visible rather than turning into a mysteriously small result.
pub fn ingest_logs(store: &mut LogStore, args: IngestLogsArgs) -> Result<Value, String> {
    let source = Path::new(&args.source);
    if !source.exists() {
        return Err(format!(
            "path does not exist: {} — pass an OTLP payload file or a directory of them",
            source.display()
        ));
    }
    let report = store
        .insert_path(source)
        .map_err(|e| format!("ingest failed for {}: {e:#}", source.display()))?;

    let mut json =
        crate::sdg::log_store::ingest_logs_json(&report, store).map_err(|e| e.to_string())?;
    json["source"] = json!(source.display().to_string());
    Ok(json)
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchLogsArgs {
    /// Restrict to one trace. Case-insensitive.
    pub trace_id: Option<String>,
    /// Restrict to one service, exactly as the exporter named it
    /// (`service.name`). Every response lists `available_services`, so a wrong
    /// guess is recoverable without a second call.
    pub service: Option<String>,
    /// Minimum severity: `Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`.
    /// Records whose severity cannot be determined never match.
    pub min_severity: Option<String>,
    /// Case-insensitive substring. Matched against the body *and* attribute
    /// values, because the .NET exporter sends the unrendered message template as
    /// the body — the interpolated values live in attributes, so a body-only
    /// search would miss the exact string someone pastes during an incident.
    pub contains: Option<String>,
    /// Search attribute values only, ignoring the body.
    pub attributes_only: Option<bool>,
    /// Only records having an attribute with this key.
    pub attribute_key: Option<String>,
    /// Inclusive lower bound, Unix nanoseconds.
    pub since_unix_nano: Option<u64>,
    /// Exclusive upper bound, Unix nanoseconds.
    pub until_unix_nano: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Search the log store.
///
/// Every filter is independent; there are no presets. The query building and the
/// response live in [`crate::sdg::log_store`] so the CLI and this tool cannot
/// drift apart in the fields that explain a short answer.
pub fn search_logs(store: &LogStore, args: SearchLogsArgs) -> Result<Value, String> {
    let query = LogQuery {
        trace_id: args.trace_id,
        service: args.service,
        min_severity: args
            .min_severity
            .as_deref()
            .map(LogQuery::parse_severity)
            .transpose()
            .map_err(|e| e.to_string())?,
        contains: args.contains,
        attributes_only: args.attributes_only.unwrap_or(false),
        attribute_key: args.attribute_key,
        since_unix_nano: args.since_unix_nano,
        until_unix_nano: args.until_unix_nano,
        only_traceable: false,
        limit: LogQuery::bounded_limit(args.limit).map_err(|e| e.to_string())?,
        offset: args.offset.unwrap_or(0),
    };
    crate::sdg::log_store::search_logs_json(store, &query).map_err(|e| e.to_string())
}
