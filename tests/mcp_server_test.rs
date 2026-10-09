//! End-to-end MCP protocol test for `tinylink serve`.
//!
//! Drives the real binary over stdio JSON-RPC against a real index built from
//! real C# sources: handshake, tool listing, and the trace→code join. The unit
//! tests cover the tool functions; this covers the wiring an agent meets first.
// Gated on the `mcp` feature rather than skipped at runtime. `serve` does not exist
// without it, so every test in here would fail rather than skip, and CI runs the suite
// both with and without default features. Compiling the file away means `cargo test`
// is green under any feature set, and the tests still run wherever the server is built.
#![cfg(feature = "mcp")]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpProcess {
    /// Start the server against a prebuilt index.
    fn start(index: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_tinylink"))
            .args(["serve", "--index", index.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn tinylink serve");

        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut me = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        me.initialize();
        me
    }

    fn initialize(&mut self) {
        self.send(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"},
            }),
        );
        self.notify("notifications/initialized", json!({}));
    }

    fn notify(&mut self, method: &str, params: Value) {
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": method, "params": params})
        )
        .expect("write notification");
        self.stdin.flush().expect("flush");
    }

    fn send(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params})
        )
        .expect("write request");
        self.stdin.flush().expect("flush");

        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).expect("read response");
            assert!(read > 0, "server closed the stream before responding");
            let value: Value = serde_json::from_str(line.trim()).unwrap_or_else(|e| {
                panic!("non-JSON line on the MCP stream ({e}): {line}");
            });
            if value.get("id").and_then(|i| i.as_i64()) == Some(self.next_id) {
                return value;
            }
        }
    }

    fn call(&mut self, name: &str, args: Value) -> Value {
        self.send("tools/call", json!({"name": name, "arguments": args}))
    }

    /// Tool payload, tolerating both MCP failure shapes: a JSON-RPC `error` or a
    /// result flagged `isError`.
    fn tool_json(&mut self, name: &str, args: Value) -> Value {
        let result = self.call(name, args);
        assert!(
            result.get("error").is_none(),
            "{name} failed at the protocol level: {result}"
        );
        let content = result["result"]["content"]
            .as_array()
            .expect("content array");
        let text = content[0]["text"].as_str().expect("text content");
        serde_json::from_str(text)
            .unwrap_or_else(|e| panic!("{name} text is not JSON ({e}): {text}"))
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Build a small project + index and return the index path.
fn setup(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("tinylink_e2e_{}", name));
    let _ = std::fs::remove_dir_all(&root);
    let agency = root.join("Agency.API");
    std::fs::create_dir_all(&agency).unwrap();
    std::fs::write(
        agency.join("Handler.cs"),
        r#"
namespace Agency.API;
public class GetMembersEndpoint
{
    public static IResult Handler(HttpContext ctx)
    {
        var result = Load();
        return Ok(result);
    }
    static IResult Load() => null;
    static IResult Ok(object r) => null;
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
    let dirs = vec![agency.display().to_string()];
    let built = tinylink::link::index::build(&dirs).expect("build index");
    tinylink::link::index::write(&built, &db).expect("write index");
    (root, db)
}

const FAILING_TRACE: &str = r#"{"resourceSpans":[
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Dashboard.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"t-e2e","spanId":"s1","parentSpanId":"","name":"GET","kind":3,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.target","value":{"stringValue":"/api/agency/9f1c/member"}}],
    "startTimeUnixNano":0,"endTimeUnixNano":100000000}]}]},
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Agency.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"t-e2e","spanId":"s2","parentSpanId":"s1","name":"GET","kind":2,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.route","value":{"stringValue":"/api/agency/{agencyId}/member"}},
                  {"key":"http.status_code","value":{"intValue":503}}],
    "startTimeUnixNano":5000000,"endTimeUnixNano":90000000}]}]}
]}"#;

#[test]
fn handshake_lists_every_tool_with_a_schema_and_an_honest_read_only_flag() {
    let (root, db) = setup("tools");
    let mut mcp = McpProcess::start(&db);

    let listed = mcp.send("tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "explain_error",
        "blast_radius",
        "find_methods",
        "list_indexed_services",
        "ingest_logs",
        "search_logs",
    ] {
        assert!(names.contains(&expected), "missing {expected} in {names:?}");
    }
    assert_eq!(tools.len(), 6, "unexpected tool set: {names:?}");

    // ingest_logs writes to the log store, so it must not claim to be read-only.
    // A client that trusts the flag to decide whether a call needs approval would
    // otherwise let a state change through unprompted.
    let writes_state = ["ingest_logs"];
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let expected_read_only = !writes_state.contains(&name);
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            json!(expected_read_only),
            "{name} has the wrong read-only annotation"
        );
        assert!(tool["description"].as_str().unwrap().len() > 40);
        assert!(
            !tool["inputSchema"].is_null(),
            "{} missing schema",
            tool["name"]
        );
    }
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn explain_error_returns_the_handler_over_the_protocol() {
    let (root, db) = setup("explain");
    let mut mcp = McpProcess::start(&db);

    let out = mcp.tool_json("explain_error", json!({"trace": FAILING_TRACE}));
    let edges = out["error_edges"].as_array().expect("error_edges");
    assert_eq!(edges.len(), 1, "{out}");
    assert_eq!(edges[0]["error_count"], 1);
    assert_eq!(
        edges[0]["to"]["method"], "GetMembersEndpoint.Handler",
        "the 503 must resolve to the handler: {out}"
    );
    assert_eq!(edges[0]["to"]["confidence"], 0.95);
    let file = edges[0]["to"]["file"].as_str().expect("file");
    assert!(file.ends_with("Handler.cs"), "got {file}");
    assert!(edges[0]["to"]["line"].as_u64().unwrap() > 0);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn blast_radius_reports_the_endpoint_over_the_protocol() {
    let (root, db) = setup("blast");
    let mut mcp = McpProcess::start(&db);

    let out = mcp.tool_json(
        "blast_radius",
        json!({"target": "GetMembersEndpoint.Handler"}),
    );
    let results = out["results"].as_array().expect("results");
    assert_eq!(results.len(), 1, "{out}");
    let routes = results[0]["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1, "{out}");
    assert_eq!(routes[0]["pattern"], "/api/agency/{agencyId}/member");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn find_methods_and_services_orient_the_agent() {
    let (root, db) = setup("orient");
    let mut mcp = McpProcess::start(&db);

    let services = mcp.tool_json("list_indexed_services", json!({}));
    let list = services["services"].as_array().expect("services");
    assert_eq!(list.len(), 1, "{services}");
    assert_eq!(list[0]["service"], "Agency.API");
    assert_eq!(list[0]["routes"], 1);

    let found = mcp.tool_json("find_methods", json!({"query": "GetMembers"}));
    assert!(found["count"].as_u64().unwrap() >= 1, "{found}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_bad_trace_is_reported_and_the_session_stays_usable() {
    let (root, db) = setup("errors");
    let mut mcp = McpProcess::start(&db);

    let bad = mcp.call("explain_error", json!({"trace": "{broken"}));
    let reported = bad.get("error").is_some() || bad["result"]["isError"] == json!(true);
    assert!(reported, "a malformed trace must be reported: {bad}");

    // The session must survive: an agent will retry with a corrected trace.
    let ok = mcp.tool_json("explain_error", json!({"trace": FAILING_TRACE}));
    assert_eq!(ok["error_edges"].as_array().unwrap().len(), 1);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn server_refuses_to_start_without_an_index() {
    let out = Command::new(env!("CARGO_BIN_EXE_tinylink"))
        .args(["serve", "--index", "/nonexistent/tinylink.db"])
        .stdin(Stdio::null())
        .output()
        .expect("run serve");

    assert!(!out.status.success(), "must not start without an index");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tinylink index"),
        "startup error should say how to build one: {stderr}"
    );
}

// ── logs, over the protocol ──

/// The capture is the raw body of two real OTLP/protobuf exports from the
/// OpenTelemetry .NET SDK, so this exercises the wire format an agent will meet
/// rather than a payload written to suit the parser.
fn capture_payload(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("agency-logs.bin");
    std::fs::write(&path, include_bytes!("fixtures/otlp-logs-capture.bin")).unwrap();
    path
}

#[test]
fn ingest_then_search_logs_over_the_protocol() {
    let (root, db) = setup("logs");
    let mut mcp = McpProcess::start(&db);
    let payload = capture_payload(&root);

    let ingested = mcp.tool_json("ingest_logs", json!({ "source": payload }));
    assert_eq!(ingested["records_inserted"], 8, "{ingested}");
    assert_eq!(ingested["payloads_read"], 1, "{ingested}");
    assert_eq!(ingested["distinct_traces"], 2, "{ingested}");
    assert_eq!(ingested["services"][0], "Agency.API", "{ingested}");

    // What blew up.
    let errors = mcp.tool_json(
        "search_logs",
        json!({ "min_severity": "Error", "limit": 10 }),
    );
    assert_eq!(errors["count"], 2, "{errors}");
    assert_eq!(errors["store_records"], 8, "{errors}");
    assert_eq!(
        errors["logs"][0]["attributes"]["exception.type"], "InvalidOperationException",
        "{errors}"
    );

    // Free text that only exists in an attribute — the case a body-only search
    // silently misses.
    let by_value = mcp.tool_json(
        "search_logs",
        json!({ "contains": "GetAgencyMembers", "limit": 10 }),
    );
    assert_eq!(by_value["count"], 2, "{by_value}");

    // The whole request for one failing trace.
    let trace = errors["logs"][0]["trace_id"].as_str().unwrap().to_string();
    let request = mcp.tool_json("search_logs", json!({ "trace_id": trace, "limit": 50 }));
    assert_eq!(request["count"], 4, "{request}");
    assert_eq!(
        request["traces_in_page"].as_array().map(Vec::len),
        Some(1),
        "{request}"
    );

    // The join the whole thing exists for. The trace tool names the failing
    // handler and line; the log tool names what that request actually logged.
    // Neither is derivable from the other.
    let explained = mcp.tool_json("explain_error", json!({ "trace": FAILING_TRACE }));
    let edge = &explained["error_edges"][0];
    assert_eq!(
        edge["to"]["method"], "GetMembersEndpoint.Handler",
        "{explained}"
    );
    assert_eq!(edge["to"]["line"], 5, "{explained}");
    assert_eq!(edge["trace_ids"][0], "t-e2e", "{explained}");

    let logged = mcp.tool_json(
        "search_logs",
        json!({ "contains": "exception.stacktrace", "limit": 10 }),
    );
    assert_eq!(logged["count"], 2, "{logged}");
}

/// An empty result has to be explainable. Without `store_records` an agent cannot
/// tell "nothing matched" from "no logs were ever ingested".
#[test]
fn an_empty_log_search_reports_what_the_store_holds() {
    let (_root, db) = setup("logs_empty");
    let mut mcp = McpProcess::start(&db);

    let out = mcp.tool_json("search_logs", json!({ "limit": 10 }));
    assert_eq!(out["count"], 0, "{out}");
    assert_eq!(out["store_records"], 0, "{out}");
    // With no filter at all, the only honest answer is "nothing is stored", and
    // the service list has to say so.
    assert_eq!(
        out["available_services"].as_array().map(Vec::is_empty),
        Some(true),
        "{out}"
    );
}

#[test]
fn search_logs_rejects_an_unknown_severity_by_name() {
    let (_root, db) = setup("logs_sev");
    let mut mcp = McpProcess::start(&db);

    // `Critical` is deliberately not the input: .NET's LogLevel.Critical maps onto
    // Fatal and is accepted. The point is a name with no mapping at all.
    let result = mcp.call("search_logs", json!({ "min_severity": "catastrophic" }));
    let message = result["error"]["message"]
        .as_str()
        .expect("an error message");
    assert!(
        message.contains("Trace") && message.contains("Fatal"),
        "the error should list the valid names: {message}"
    );
}

#[test]
fn ingest_logs_reports_a_missing_source_instead_of_creating_an_empty_store() {
    let (_root, db) = setup("logs_missing");
    let mut mcp = McpProcess::start(&db);

    let result = mcp.call("ingest_logs", json!({ "source": "/nonexistent/payloads" }));
    let message = result["error"]["message"]
        .as_str()
        .expect("an error message");
    assert!(message.contains("does not exist"), "{message}");
}

/// One payload per file, and a bad file must not cost the good ones. This is the
/// ordinary case of pointing `ingest_logs` at a directory of collector dumps.
#[test]
fn ingesting_a_directory_reports_a_bad_file_and_keeps_the_rest() {
    let (root, db) = setup("logs_dir");
    let mut mcp = McpProcess::start(&db);

    let dir = root.join("payloads");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a-good.bin"),
        include_bytes!("fixtures/otlp-logs-capture.bin"),
    )
    .unwrap();
    std::fs::write(dir.join("b-broken.json"), b"{ not json").unwrap();

    let out = mcp.tool_json("ingest_logs", json!({ "source": dir }));
    assert_eq!(out["records_inserted"], 8, "{out}");
    assert_eq!(out["payloads_read"], 1, "{out}");
    let problems = out["problems"].as_array().expect("problems");
    assert!(
        problems
            .iter()
            .any(|p| p.as_str().unwrap().contains("b-broken")),
        "{out}"
    );
}
