//! End-to-end coverage for the minimal-API route style that most modern .NET
//! services use: one-shot endpoint classes registered as
//! `MapGet("/path", Type.Handler)`.
//!
//! These tests build real C# sources on disk, so they exercise the full path —
//! parse → symbol table → route extraction → code graph → unified mapping —
//! rather than hand-built graphs. That path was previously untested: the C#
//! loader and route extractor had no fixtures at all, and the `Class.Handler`
//! idiom silently produced zero routes.

use std::fs;
use std::path::{Path, PathBuf};

use tinylink::code::loader::{load_project, load_routes};
use tinylink::link::builder;

/// Create a throwaway directory unique to the calling test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tinylink_it_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write source");
}

/// Two services, each with one-shot minimal-API endpoints.
fn write_two_services(root: &Path) {
    write(
        root,
        "Agency.API/Endpoints/Members/GetMembersEndpoint.cs",
        r#"
namespace Agency.API.Endpoints.Members;

public class GetMembersEndpoint
{
    public static async Task<IResult> Handler(HttpContext ctx)
    {
        var id = ctx.Request.RouteValues["agencyId"];
        return Results.Ok();
    }
}
"#,
    );
    write(
        root,
        "Agency.API/Extensions/EndpointExtension.cs",
        r#"
using Agency.API.Endpoints.Members;

namespace Agency.API.Extensions;

public static class EndpointExtension
{
    public static IEndpointRouteBuilder MapAgencyApi(this IEndpointRouteBuilder app)
    {
        var agencyApi = app.MapGroup("").WithResourceAuthorization();

        agencyApi.MapGet("/api/agency/{agencyId:guid}/member",
            GetMembersEndpoint.Handler);
        agencyApi.MapPost("/api/agency",
            CreateAgencyEndpoint.Handler)
            .WithValidation<CreateAgencyEndpoint.Request>();

        return app;
    }
}
"#,
    );
    write(
        root,
        "Agency.API/Endpoints/Agency/CreateAgencyEndpoint.cs",
        r#"
namespace Agency.API.Endpoints.Agency;

public class CreateAgencyEndpoint
{
    public static async Task<IResult> Handler(HttpContext ctx)
    {
        return Results.Ok();
    }
}
"#,
    );
    write(
        root,
        "Billing.API/Endpoints/GetInvoiceEndpoint.cs",
        r#"
namespace Billing.API.Endpoints;

public class GetInvoiceEndpoint
{
    public static async Task<IResult> Handler(HttpContext ctx)
    {
        return Results.Ok();
    }
}
"#,
    );
    write(
        root,
        "Billing.API/Extensions/EndpointExtension.cs",
        r#"
using Billing.API.Endpoints;

namespace Billing.API.Extensions;

public static class EndpointExtension
{
    public static IEndpointRouteBuilder MapBillingApi(this IEndpointRouteBuilder app)
    {
        var billingApi = app.MapGroup("");
        billingApi.MapGet("/api/billing/{accountId}/invoice", GetInvoiceEndpoint.Handler);
        return app;
    }
}
"#,
    );
}

/// An OTLP trace with one failing cross-service hop.
fn write_error_trace(dir: &Path) -> PathBuf {
    let path = dir.join("trace.json");
    fs::write(
        &path,
        r#"{"resourceSpans":[
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Dashboard.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"trace-1","spanId":"s1","parentSpanId":"","name":"GET","kind":3,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.target","value":{"stringValue":"/api/agency/9f1c-2222/member"}}],
    "startTimeUnixNano":0,"endTimeUnixNano":90000000}]}]},
 {"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"Agency.API"}}]},
  "scopeSpans":[{"spans":[
   {"traceId":"trace-1","spanId":"s2","parentSpanId":"s1","name":"GET","kind":2,
    "attributes":[{"key":"http.method","value":{"stringValue":"GET"}},
                  {"key":"http.route","value":{"stringValue":"/api/agency/{agencyId}/member"}},
                  {"key":"http.status_code","value":{"intValue":500}}],
    "startTimeUnixNano":5000000,"endTimeUnixNano":80000000}]}]}
]}"#,
    )
    .expect("write trace");
    path
}

#[test]
fn test_solution_root_is_split_into_per_project_services() {
    use tinylink::link::builder::{expand_solution_roots, project_dirs_in};

    let root = temp_dir("solution");
    let agency = root.join("Agency.API");
    let billing = root.join("Billing.API");
    let shared = root.join("Shared.Common");
    std::fs::create_dir_all(&agency).unwrap();
    std::fs::create_dir_all(&billing).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    for (dir, name) in [
        (&agency, "Agency"),
        (&billing, "Billing"),
        (&shared, "Common"),
    ] {
        std::fs::write(dir.join(format!("{name}.csproj")), "<Project/>").unwrap();
    }
    // A stray directory with no .csproj must not be mistaken for a project.
    std::fs::create_dir_all(root.join("docs")).unwrap();

    // Each service needs real sources, otherwise there are no routes to tag.
    std::fs::write(
        agency.join("Endpoint.cs"),
        "namespace Agency.API;\npublic class AgencyEndpoint { public static IResult Handler() => null; }\n",
    )
    .unwrap();
    std::fs::write(
        agency.join("Ext.cs"),
        "app.MapGet(\"/api/agency/member\", AgencyEndpoint.Handler);\n",
    )
    .unwrap();
    std::fs::write(
        billing.join("Endpoint.cs"),
        "namespace Billing.API;\npublic class BillingEndpoint { public static IResult Handler() => null; }\n",
    )
    .unwrap();
    std::fs::write(
        billing.join("Ext.cs"),
        "app.MapGet(\"/api/billing/invoice\", BillingEndpoint.Handler);\n",
    )
    .unwrap();

    let found = project_dirs_in(root.to_str().unwrap());
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().any(|d| d.ends_with("Agency.API")), "{found:?}");
    assert!(
        found.iter().any(|d| d.ends_with("Billing.API")),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|d| d.ends_with("docs")),
        "a directory without a .csproj is not a project: {found:?}"
    );

    // And services end up tagged with their own names.
    let (code, routes) =
        builder::load_merged_projects(root.to_str().unwrap()).expect("load solution root");
    let route_services: Vec<&str> = routes
        .entries
        .iter()
        .filter_map(|r| r.service.as_deref())
        .collect();
    assert!(
        route_services.contains(&"Agency.API"),
        "routes must carry their own service name: {route_services:?}"
    );
    assert!(code
        .methods
        .iter()
        .any(|m| m.service.as_deref() == Some("Agency.API")));

    let expanded = expand_solution_roots(&[root.to_str().unwrap().to_string()]);
    assert_eq!(expanded.len(), 3, "{expanded:?}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn test_single_project_directory_is_not_split() {
    use tinylink::link::builder::project_dirs_in;

    let root = temp_dir("single_project");
    let svc = root.join("Agency.API");
    std::fs::create_dir_all(&svc).unwrap();
    std::fs::write(svc.join("Agency.API.csproj"), "<Project/>").unwrap();
    std::fs::write(
        svc.join("Handler.cs"),
        "namespace A;\npublic class H { public static void M() {} }\n",
    )
    .unwrap();

    let found = project_dirs_in(svc.to_str().unwrap());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].ends_with("Agency.API"));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn test_loose_cs_directory_loads_as_one_service() {
    use tinylink::link::builder::project_dirs_in;

    let root = temp_dir("loose");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("A.cs"), "namespace A;\npublic class A { }\n").unwrap();

    let found = project_dirs_in(root.to_str().unwrap());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0], root.to_str().unwrap());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn test_same_class_and_method_in_two_services_are_both_kept() {
    use tinylink::code::types::{CodeGraph, MethodNode};

    let mut g = CodeGraph::new();
    for service in ["Agency.API", "Billing.API"] {
        g.add_method(MethodNode {
            class: "SharedHandler".into(),
            method: "Handle".into(),
            signature: "void Handle()".into(),
            file: "H.cs".into(),
            line_start: 1,
            line_end: 5,
            service: Some(service.into()),
        });
    }
    assert_eq!(
        g.methods.len(),
        2,
        "de-duplicating on class+method alone drops one service's handler"
    );
    assert_eq!(
        g.find_method_in("SharedHandler", "Handle", "Agency.API")
            .unwrap()
            .service
            .as_deref(),
        Some("Agency.API")
    );
    assert_eq!(
        g.find_method_in("SharedHandler", "Handle", "Billing.API")
            .unwrap()
            .service
            .as_deref(),
        Some("Billing.API")
    );
}

#[test]
fn test_minimal_api_routes_are_extracted() {
    let root = temp_dir("routes");
    write_two_services(&root);

    let agency = root.join("Agency.API");
    let (table, inline_lambdas) = load_routes(agency.to_str().unwrap()).expect("extract routes");
    assert_eq!(inline_lambdas, 0, "no route in this fixture uses a lambda");

    let get = table
        .entries
        .iter()
        .find(|r| r.http_method == "GET")
        .expect("GET route");
    // `{agencyId:guid}` must be normalized for segment matching.
    assert_eq!(get.route_pattern, "/api/agency/{agencyId}/member");
    assert_eq!(get.class, "GetMembersEndpoint");
    assert_eq!(get.method, "Handler");

    let post = table
        .entries
        .iter()
        .find(|r| r.http_method == "POST")
        .expect("POST route");
    assert_eq!(post.route_pattern, "/api/agency");
    assert_eq!(post.class, "CreateAgencyEndpoint");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_chained_call_registers_route_once() {
    let root = temp_dir("chain");
    write_two_services(&root);

    let agency = root.join("Agency.API");
    let (table, _) = load_routes(agency.to_str().unwrap()).expect("extract routes");
    let posts: Vec<_> = table
        .entries
        .iter()
        .filter(|r| r.http_method == "POST")
        .collect();
    assert_eq!(
        posts.len(),
        1,
        ".WithValidation<T>() on a chained call must not duplicate the route"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_loaded_methods_report_their_file_and_line() {
    let root = temp_dir("files");
    write_two_services(&root);

    let cg = load_project(root.join("Agency.API").to_str().unwrap()).expect("load project");

    let m = cg
        .find_method("GetMembersEndpoint", "Handler")
        .expect("handler method present");
    assert!(
        !m.file.is_empty(),
        "method file must be populated, not an empty string"
    );
    assert!(m.file.ends_with("GetMembersEndpoint.cs"), "got {}", m.file);
    assert!(m.line_start > 0, "line_start must be set");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_server_side_error_is_reported() {
    let root = temp_dir("error");
    write_two_services(&root);
    let trace = write_error_trace(&root);

    // The 500 is recorded on the SERVER span only. Reading the client span
    // alone missed it and `query` reported "No errors found".
    let errors = builder::query_errors(
        trace.to_str().unwrap(),
        root.join("Agency.API").to_str().unwrap(),
        None,
    )
    .expect("query errors");

    assert_eq!(
        errors.len(),
        1,
        "a server-recorded 500 must surface as an error edge"
    );
    assert!(errors[0].error);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_endpoint_resolves_to_handler_with_file_and_line() {
    let root = temp_dir("resolve");
    write_two_services(&root);
    let trace = write_error_trace(&root);

    let unified = builder::build_unified(
        trace.to_str().unwrap(),
        root.join("Agency.API").to_str().unwrap(),
    )
    .expect("build unified");

    let edge = unified
        .edges
        .iter()
        .find(|e| e.target_service == "Agency.API")
        .expect("edge into Agency.API");

    assert_eq!(
        edge.target_method.as_deref(),
        Some("GetMembersEndpoint.Handler"),
        "route must resolve to the endpoint class, not the registrar"
    );
    assert_eq!(edge.target_confidence, Some(0.95));
    assert_eq!(edge.trace_id.as_deref(), Some("trace-1"));
    let file = edge.target_file.as_deref().expect("target_file set");
    assert!(file.ends_with("GetMembersEndpoint.cs"), "got {file}");
    assert!(edge.target_line.unwrap_or(0) > 0);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_tracepath_filter_by_trace_id_finds_the_edge() {
    let root = temp_dir("tracepath");
    write_two_services(&root);
    let trace = write_error_trace(&root);
    let agency = root.join("Agency.API");

    let matching = builder::build_trace_path(
        trace.to_str().unwrap(),
        agency.to_str().unwrap(),
        Some("trace-1"),
    )
    .expect("trace path");
    assert_eq!(
        matching.len(),
        1,
        "filtering by a trace id present in the data must return its edge"
    );

    let absent = builder::build_trace_path(
        trace.to_str().unwrap(),
        agency.to_str().unwrap(),
        Some("no-such-trace"),
    )
    .expect("trace path");
    assert!(absent.is_empty(), "unknown trace id must return nothing");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_services_do_not_borrow_each_others_handlers() {
    let root = temp_dir("scoped");
    write_two_services(&root);
    let (code, routes) = builder::load_merged_projects(&format!(
        "{},{}",
        root.join("Agency.API").display(),
        root.join("Billing.API").display()
    ))
    .expect("load merged projects");

    // Both classes define a `Handler`, so an unscoped route match could return
    // either one. Each service must resolve to its own.
    let agency_get = routes
        .match_best("GET", "/api/agency/9f1c/member")
        .expect("agency route");
    assert_eq!(agency_get.class, "GetMembersEndpoint");

    let billing_get = routes
        .match_best("GET", "/api/billing/ab12/invoice")
        .expect("billing route");
    assert_eq!(billing_get.class, "GetInvoiceEndpoint");

    // The merged code graph carries both, each tagged with its service.
    assert!(code
        .methods
        .iter()
        .any(|m| m.class == "GetMembersEndpoint" && m.service.as_deref() == Some("Agency.API")));
    assert!(code
        .methods
        .iter()
        .any(|m| m.class == "GetInvoiceEndpoint" && m.service.as_deref() == Some("Billing.API")));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_inline_lambda_route_is_reported_not_invented() {
    let root = temp_dir("lambda");
    write(
        &root,
        "Lambda.API/Extensions/Endpoints.cs",
        r#"
var app = WebApplication.CreateBuilder(args).Build();
app.MapGet("/cost", async (HttpContext ctx) => { return Results.Ok(); });
"#,
    );

    let (table, inline_lambdas) =
        load_routes(root.join("Lambda.API").to_str().unwrap()).expect("extract routes");
    assert_eq!(inline_lambdas, 1, "lambda route must be counted");
    assert!(
        table.entries.is_empty(),
        "a lambda has no named method, so it must not become a route entry"
    );

    fs::remove_dir_all(&root).ok();
}
