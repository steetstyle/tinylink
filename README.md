# tinylink

Links OpenTelemetry service graphs (SDG) to code-level dependence graphs (PDG):
you point it at an OTLP trace and a set of C# projects, and it tells you which
method handled which request — including for requests that failed.

```
tinylink link trace.json ./Agency.API,./Billing.API --format dot
```

## Install

```bash
cargo build --release
# binary at target/release/tinylink
```

Requires a Rust toolchain and network access on first build (it depends on
[tiny-pdg-cs](https://github.com/steetstyle/tinypdg-cs) via git).

## Try it on a real solution

`example-projects/` holds a helper that fetches a large, real ASP.NET Core
solution so you can exercise the tool end to end:

```bash
./example-projects/fetch.sh          # clones unicpeak-analytics-api
```

That solution has 1667 C# files across 92 projects and uses minimal-API
endpoints, which is the style most modern services use.

```bash
R=example-projects/unicpeak-analytics-api

# routes discovered per service
for s in Agency.API Billing.API Dashboard.API; do
  echo "$s: $(cargo run -q -- route $R/$s 2>/dev/null | grep -cE '^\s+(GET|POST|PUT|DELETE|PATCH)')"
done
```

## Commands

| Command | Purpose |
| --- | --- |
| `sdg <trace>` | Build the service dependency graph from OTLP JSON |
| `code <source>` | Extract the code graph and routes from a C# project |
| `link <trace> <sources>` | Unify both into one graph |
| `tracepath <trace> <sources>` | Walk a request hop by hop |
| `query <trace> <sources>` | List error edges and the methods behind them |
| `pdg <source>` | Emit the program dependence graph for a method |
| `explore <trace> <sources>` | Cross-service exploration with drill-down |
| `index <sources> -o <file>` | Build a reusable SQLite index |
| `serve --index <file>` | Serve trace→code linking over MCP |
| `logs-ingest <path>` | Load OTLP log payloads into the log store |
| `logs-search` | Query the log store |
| `route <source>` | List discovered HTTP routes (via tiny-pdg-cs) |
| `prometheus <url>` | Build the SDG from Prometheus metrics (`online` feature) |

`sources` is a comma-separated list of project directories. A **solution root**
is expanded into its constituent projects, so services are tagged with their own
directory names (`Agency.API`) rather than the solution folder.

```bash
tinylink query trace.json ./Agency.API,./Billing.API --pdg
tinylink pdg ./Agency.API --class GetMembersEndpoint --method Handler
tinylink explore trace.json ./Agency.API --pdg
```

`--format json` (the default) writes machine-readable output to stdout and a
summary to stderr, so piping into `jq` works.

## Persistent index and the MCP server

Parsing 1667 C# files takes ~1.9s. Fine for a one-shot CLI call, unusable for a
long-running process. `index` parses once into a SQLite file:

```bash
tinylink index ./MySolution -o tinylink.db      # 1.9s
tinylink index ./MySolution -o tinylink.db      # 0.01s, sources unchanged
```

Rebuilding is skipped when the sources are byte-identical, so this is cheap to
run on every CI commit. The index holds static code metadata only — methods with
file and line, HTTP routes, call edges. Traces and logs are per-incident data and
are deliberately not indexed.

`serve` exposes the trace→code join over MCP for an agent:

```bash
cargo build --release --features mcp
tinylink serve --index tinylink.db
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
tinylink logs-ingest ./captures          # a file, or a directory, one payload per file
tinylink logs-search --min-severity Error
tinylink logs-search --trace-id 4bf92f3577b34da6a3ce929d0e0e4736
```

The store defaults to `TINYLINK_LOG_STORE`, else `<index>.logs.db`, so a CLI
ingest and a server query can be pointed at one file.

Two things about the data are worth knowing before you filter it. Both come from
reading a real `AddOtlpExporter` capture rather than the schema:

- **The body is the unrendered message template.** A log exports as
  `Handler {HandlerName} failed` with the values in separate attributes. Free-text
  search therefore covers attribute values too — otherwise the string you paste in
  during an incident would not match the log that contains it.
- **The .NET SDK sends gRPC, not HTTP.** `AddOtlpExporter` with only an
  `Endpoint` set talks to 4317 over gRPC. Capturing from a collector, or setting
  `Protocol = OtlpExportProtocol.HttpProtobuf`, is what makes the payloads
  readable.

Both OTLP/JSON and OTLP/protobuf are accepted, gzipped or not, detected by
sniffing the first bytes rather than trusting a header or a file extension.

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
matching pattern — which, at the highest confidence, is a confident wrong
answer.

Route templates are normalized before matching: `{id:guid}`, `{id?}`, `{*path}`
and `{**path}` all become `{name}`, so `{agencyId:guid}` matches a concrete
request path by segment count.

Both MVC controllers and minimal APIs are supported, including the one-shot
idiom where the handler is a static method on a dedicated endpoint class:

```csharp
agencyApi.MapGet("/api/agency/{agencyId:guid}/member", GetMembersEndpoint.Handler);
```

Routes whose handler is an inline lambda have no named method to point at. They
are counted and reported rather than turned into entries pointing nowhere.

## Tests

```bash
cargo test
```

Two tests in `tests/traverse_test.rs` need an eShop checkout and are `#[ignore]`d;
they report as ignored rather than passing without running anything. To enable:

```bash
git clone --depth 1 https://github.com/dotnet/eShopOnWeb.git /tmp/eshop
cargo test --test traverse_test -- --ignored
```

The Prometheus path has a Docker-based live test:

```bash
bash tests/docker/prometheus/test_live.sh
```

## License

MIT OR Apache-2.0
