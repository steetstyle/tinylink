# tinylink

Links OpenTelemetry service graphs (SDG) to code-level dependence graphs (PDG):
you point it at an OTLP trace and a set of C# projects, and it tells you which
method handled which request — including for requests that failed.

```
tinylink link trace.json ./Agency.API,./Billing.API --format dot
```

An incident gives you a trace id and a service name. That is not a file and a
line. tinylink resolves the hop that failed to the method that threw, with the
file and the line, and reports how confident the mapping is.

## What it does

Twelve CLI commands:

```
sdg          Build service dependency graph from OTLP trace data
code         Analyze C# source and extract the code graph (via tiny-pdg-cs)
link         Unify SDG and code graph into a single graph
tracepath    Walk a single request through services and code
query        List error edges and the methods behind them
prometheus   Build the SDG from Prometheus service-graph metrics (online feature)
index        Build a reusable SQLite index of code, routes and call edges
serve        Serve trace→code linking over MCP for AI agents
logs-ingest  Ingest OTLP log payloads into the log store
logs-search  Search the log store
pdg          Emit the program dependence graph for a method
explore      Cross-service exploration with drill-down
```

`route` is not one of these — HTTP route extraction belongs to
[tiny-pdg-cs](https://github.com/steetstyle/tinypdg-cs), and tinylink calls into
it rather than keeping a second copy. `tiny-pdg-cs route ./Agency.API` is the
command you want.

## Install

```bash
cargo build --release
# binary at target/release/tinylink
```

Requires a Rust toolchain and network access on first build: tinylink depends on
tiny-pdg-cs through git, so the C# parser is not vendored.

### Feature flags

| Feature | Default | Adds |
| --- | --- | --- |
| `online` | yes | `prometheus` — `reqwest` + `tokio` |
| `mcp` | no | `serve` over stdio |
| `mcp-http` | no | `serve --http` — Streamable HTTP |

`mcp-http` implies `mcp`. The index and every CLI command work without either;
a tokio runtime is only needed to serve.

```bash
cargo build --release                      # CLI only
cargo build --release --features mcp       # serve over stdio
cargo build --release --features mcp-http  # serve over HTTP too
```

## Try it on a real solution

`example-projects/fetch.sh` pulls a large, real ASP.NET Core solution so you can
exercise the tool end to end. It has 1667 C# files across 92 projects and uses
minimal-API endpoints, the style most modern services use.

```bash
./example-projects/fetch.sh          # clones unicpeak-analytics-api
```

```bash
R=example-projects/unicpeak-analytics-api

# routes per service — this is tiny-pdg-cs, called through its own CLI
for s in Agency.API Billing.API Dashboard.API; do
  echo "$s: $(tiny-pdg-cs route $R/$s | grep -cE '^\s+(GET|POST|PUT|DELETE|PATCH)')"
done
```

```
Agency.API: 14
Billing.API: 11
Dashboard.API: 53
```

A solution root is expanded into its constituent projects, so services are tagged
with their own directory names rather than the solution folder:

```bash
tinylink index example-projects/unicpeak-analytics-api -o tinylink.db
# Indexed 4464 methods, 186 routes, 17475 call edges across 1 project(s)
#   ⚠ 16 route(s) use an inline lambda and have no named handler
```

`across 1 project(s)` counts the *argument* you passed, not the projects found —
one solution root is one argument. The expansion is visible in the result: the
route table comes back tagged per service (`Agency.API`, `Dashboard.API`, …)
rather than with the solution folder's name, which is what lets a trace naming
`Agency.API` match. Pass explicit directories and the count follows:

```bash
tinylink index ./Agency.API,./Billing.API -o tinylink.db
# Indexed 43 methods, 25 routes, 147 call edges across 2 project(s)
```

## From a trace to a line

Given a trace where `Agency.API` returns 500:

```
$ tinylink query trace.json ./Agency.API
Error Paths (1 edge(s)):
#      From                 To                   Method                         File:Line
------------------------------------------------------------------------------------------
  1    Dashboard.API      → Agency.API         GetAgencyMembersEndpoint.Handler  Agency.API/Endpoints/Members/GetAgencyMembersEndpoint.cs:14
```

The route in the trace is `/api/agency/9f3a/member`, a concrete request path. The
code has `/api/agency/{agencyId}/member`. Matching the two is what turns a trace
into a method.

```bash
$ tinylink tracepath trace.json ./Agency.API
Trace Path (1 steps):
#      Source               → Target                       Method               Error    Latency
----------------------------------------------------------------------------------------------------
  1    Dashboard.API      → Agency.API                   GetAgencyMembersEndpoint.Handler ⚠      350   ms  Agency.API/Endpoints/Members/GetAgencyMembersEndpoint.cs:14
```

`tracepath` adds per-hop latency, so the slow hop and the failing hop can be
different hops — which is the usual case and the reason the two commands exist
separately.

## Persistent index and the MCP server

Parsing 1667 C# files takes ~2.0s. Fine for a one-shot CLI call, unusable for a
long-running process. `index` parses once into a SQLite file:

```bash
$ time tinylink index ./MySolution -o tinylink.db      # cold
Indexed 4464 methods, 186 routes, 17475 call edges
real    0m2.025s

$ time tinylink index ./MySolution -o tinylink.db      # sources unchanged
Index is up to date (tinylink.db); use --force to rebuild.
real    0m0.003s
```

The rebuild is skipped on a byte-identical source tree, keyed by a BLAKE3 content
hash, so this is cheap enough to run on every CI commit. The index holds static
code metadata only — methods with file and line, HTTP routes, call edges. Traces
and logs are per-incident data and are deliberately not indexed.

`serve` exposes the trace→code join over MCP:

```bash
tinylink serve --index tinylink.db                    # stdio
tinylink serve --index tinylink.db --http --port 8082 # Streamable HTTP
```

| Tool | Answers |
| --- | --- |
| `explain_error` | which method, file and line handled a failing request, and how confident |
| `blast_radius` | what endpoints reach a method, and its callers/callees |
| `find_methods` | where is this class/method really defined |
| `list_indexed_services` | which services and endpoints exist |
| `ingest_logs` | load OTLP log payloads into the log store (writes) |
| `search_logs` | what a trace, service or error string actually logged |

### Logs

Traces arrive per call, logs do not: they are loaded once and then queried.

```bash
$ tinylink logs-ingest tests/fixtures/otlp-logs-capture.bin
Ingested 8 record(s) from 1 payload(s) into tinylink.logs.db
  distinct traces:  2
  distinct services: 1
```

```bash
$ tinylink logs-search --min-severity Error --limit 1
[Error] 1791481751057570600 Agency.API — Handler {HandlerName} failed: {Message}
    trace e5ffc66c9d01048cb051634c519ae950
    HandlerName = GetAgencyMembers
    Message = no name for agency 9f1c
    Service = agency
    exception.message = no name for agency 9f1c
    exception.type = InvalidOperationException
```

Read that output carefully: **the body is the unrendered message template.** The
`AddOtlpExporter` capture exports `Handler {HandlerName} failed: {Message}` and
puts the values in separate attributes. Free-text search therefore covers
attribute values too — otherwise the string you paste in during an incident
(`no name for agency 9f1c`) would not match the log that contains it, because
the log body says `{Message}`.

The second thing worth knowing is that **the .NET SDK sends gRPC on 4317**, not
HTTP, unless you set `Protocol = OtlpExportProtocol.HttpProtobuf`. Capturing
from a collector is the other way to get readable payloads.

Both OTLP/JSON and OTLP/protobuf are accepted, gzipped or not, detected by
sniffing the first bytes rather than trusting a header or a file extension.

The store defaults to `TINYLINK_LOG_STORE`, else `<index>.logs.db`, so a CLI
ingest and a server query can be pointed at one file.

Useful flags beyond the obvious:

```bash
tinylink logs-search --contains "no name for"   # body and attribute values
tinylink logs-search --attributes-only --attribute-key exception.type
tinylink logs-search --since <unix_nanos> --until <unix_nanos>
tinylink logs-search --format json | jq '.records'
```

Point any MCP client at the binary over stdio. For a fuller setup — an agent
investigating incidents end to end — see
[tiny-rca](https://github.com/steetstyle/tiny-rca), which drives this server and
[tiny-pdg-cs](https://github.com/steetstyle/tinypdg-cs) together.

## How mapping works

Each service edge is matched to code by trying, in order:

| Confidence | Strategy |
| --- | --- |
| 0.95 | Route template match, **restricted to the calling service** |
| 0.85 | gRPC `service/Method` → class method |
| 0.70 | Route template match against another service's routes |
| 0.60 | Last path segment matched against method names in that service |
| 0.40 | Service name matched against class name |

The route table is scoped per service on purpose. In a multi-project load the
tables are merged, and an unscoped match returns whichever service registered a
matching pattern — which, at the highest confidence, is a confident wrong answer.

Route templates are normalized before matching: `{id:guid}`, `{id?}`, `{*path}`
and `{**path}` all become `{name}`, so `{agencyId:guid}` matches a concrete
request path by segment count.

Both MVC controllers and minimal APIs are supported, including the one-shot idiom
where the handler is a static method on a dedicated endpoint class:

```csharp
agencyApi.MapGet("/api/agency/{agencyId:guid}/member", GetMembersEndpoint.Handler);
```

Routes whose handler is an inline lambda have no named method to point at. They
are counted and reported rather than turned into entries pointing nowhere.

## Program dependence graphs

Drill from a method to the statements its branches depend on:

```bash
$ tinylink pdg ./Agency.API --class GetAgencyMembersEndpoint --method Handler
digraph PDG {
  rankdir=TB;
  node [shape=box style=rounded];

  subgraph cluster__77b62fe8a5c5be59 {
    label="Endpoints/Members/GetAgencyMembersEndpoint.cs: GetAgencyMembersEndpoint.Handler";
    labelloc=t;

    n0 [label="L14-44:Entry" style=filled fillcolor=green];
    n1 [label="L14-44:Exit"  style=filled fillcolor=red];
    n2 [label="L23-28:Statement" style=filled fillcolor=lightblue];
    n3 [label="L30-43:Statement" style=filled fillcolor=lightblue];

    n2 -> n3 [style=solid color=black];
```

The construction of the graph itself lives in tiny-pdg-cs; tinylink is the join.

## Project structure

```
src/
  cli/          clap commands — thin dispatch, no business logic
  code/         C# loading and route extraction (delegates to tiny-pdg-cs)
  link/         the SDG↔PDG join: mapper, builder, SQLite index
  sdg/          service graph: OTLP JSON, Prometheus, log OTLP and the log store
  mcp/          MCP server: tool definitions and the HTTP transport
  lib.rs        library entry point
  main.rs       binary entry point
tests/
  fixtures/otel/            OTLP trace fixtures
  fixtures/otlp-logs-capture.bin   a real AddOtlpExporter capture
  docker/prometheus/         live Prometheus test
```

`link/index.rs` is the SQLite index; `sdg/log_store.rs` is a second, separate
store for logs, on purpose — the index is rebuilt from source, the log store is
appended to during an incident, and mixing them would mean losing logs every time
someone reindexed.

## Requirements

- Rust 2021 edition (tested on stable)
- tiny-pdg-cs via git — tree-sitter 0.25 / tree-sitter-c-sharp 0.23, pinned in
  lockstep here because the PDG drill-down walks the C# syntax tree directly
- SQLite is compiled from source (`rusqlite` bundled), so there is no system
  dependency to install

## Tests

```bash
cargo test --features mcp-http
```

**176 tests.** The `--features` flag is not optional for a green run: the 11
MCP protocol tests need the `mcp` feature, and without it they fail with
`` `serve` requires the `mcp` feature `` — which reads like a bug in the server
rather than a missing flag in the command.

Two tests in `tests/traverse_test.rs` need an eShop checkout and are `#[ignore]`d;
they report as ignored rather than passing without running anything:

```bash
git clone --depth 1 https://github.com/dotnet/eShopOnWeb.git /tmp/eshop
cargo test --test traverse_test --features mcp-http -- --ignored
```

The Prometheus path has a Docker-based live test:

```bash
bash tests/docker/prometheus/test_live.sh
```

Log handling is tested against `tests/fixtures/otlp-logs-capture.bin`, which is a
real capture from a .NET program using `AddOtlpExporter` — not a hand-written
payload. The two quirks documented above were found by decoding it.

## License

MIT OR Apache-2.0