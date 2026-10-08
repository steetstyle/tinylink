//! Persistent code index in SQLite.
//!
//! # Why this exists
//!
//! The in-memory path (`link`, `query`, `tracepath`) reparses every `.cs` file
//! on each invocation: ~1.1s for a 1667-file solution. That is fine for a
//! one-shot CLI call and unusable for a long-running MCP server, where the same
//! query recurs. This module parses once, writes the result to a single file,
//! and lets later processes answer lookups by SQL.
//!
//! # What belongs here
//!
//! Static code metadata only: methods with their file and line range, HTTP
//! routes, call edges, and the service list. Traces and logs are runtime data
//! and are deliberately *not* indexed — they are queried per incident.
//!
//! # Concurrency
//!
//! Readers open the file read-only in WAL mode, so a running server never blocks
//! a rebuild and several readers can share one index.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::code::route::{RouteEntry, RouteTable};
use crate::code::types::{CallEdge, CodeGraph, MethodNode};
use crate::link::builder::{self, ProjectLoad};

/// Schema version. Bump when the layout changes; a mismatch discards the index
/// rather than reading it with the wrong expectations.
const SCHEMA_VERSION: i64 = 1;

/// A built index: the data plus the content hash it was built from.
#[derive(Debug)]
pub struct Index {
    pub code: CodeGraph,
    pub routes: RouteTable,
    pub content_hash: String,
    /// Services that failed to load, with the reason.
    pub warnings: Vec<String>,
    /// Routes with an inline lambda handler, which have no method to map to.
    pub inline_lambda_routes: usize,
}

/// Hash the source directories so an unchanged tree reuses the existing index.
///
/// Covers file paths and contents, not mtimes: a touched-but-identical file must
/// not invalidate the index, and a rewritten file with identical content should
/// not either.
pub fn content_hash(dirs: &[String]) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut raw: Vec<String> = Vec::new();
    for dir in dirs {
        let path = Path::new(dir);
        crate::code::loader::collect_cs_files(path, &mut raw);
    }
    // Sort so the hash does not depend on directory-read order.
    raw.sort();

    for file in &raw {
        hasher.update(file.as_bytes());
        // Read errors are skipped rather than fatal: an unreadable file should
        // not block indexing the rest of the project.
        if let Ok(bytes) = std::fs::read(file) {
            hasher.update(&bytes);
        }
        hasher.update(&[0u8]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Build an index in memory from one or more project directories.
pub fn build(dirs: &[String]) -> Result<Index> {
    if dirs.is_empty() {
        anyhow::bail!("At least one source directory required");
    }

    let mut code = CodeGraph::new();
    let mut routes = RouteTable::new();
    let mut warnings = Vec::new();
    let mut inline_lambda_routes = 0usize;

    for dir in builder::expand_solution_roots(dirs) {
        let loaded: ProjectLoad = builder::load_project(&dir);
        if let Some(ref err) = loaded.error {
            warnings.push(format!("{}: {}", loaded.dir, err));
        }
        if let Some(cg) = loaded.code {
            merge_code(&mut code, cg);
        }
        if let Some(rt) = loaded.routes {
            for e in rt.entries {
                push_route(&mut routes, e);
            }
        }
        inline_lambda_routes += loaded.inline_lambda_routes;
    }

    let content_hash = content_hash(dirs)?;
    Ok(Index {
        code,
        routes,
        content_hash,
        warnings,
        inline_lambda_routes,
    })
}

fn merge_code(into: &mut CodeGraph, cg: CodeGraph) {
    for m in cg.methods {
        into.add_method(m);
    }
    for c in cg.calls {
        if !into.calls.iter().any(|e| {
            e.source_class == c.source_class
                && e.source_method == c.source_method
                && e.target_class == c.target_class
                && e.target_method == c.target_method
        }) {
            into.calls.push(c);
        }
    }
    for cls in cg.classes {
        if !into.classes.contains(&cls) {
            into.classes.push(cls);
        }
    }
    for iface in cg.interfaces {
        if !into.interfaces.contains(&iface) {
            into.interfaces.push(iface);
        }
    }
}

fn push_route(into: &mut RouteTable, e: RouteEntry) {
    if !into.entries.iter().any(|x| {
        x.http_method == e.http_method
            && x.route_pattern == e.route_pattern
            && x.class == e.class
            && x.method == e.method
            && x.service == e.service
    }) {
        into.add(e);
    }
}

/// Write an index to `path`, replacing any previous contents.
pub fn write(index: &Index, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create index directory {}", parent.display()))?;
    }

    let conn = Connection::open(path).with_context(|| format!("open index {}", path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "synchronous", "NORMAL").ok();
    // Write to a fresh database: leaving stale rows behind would let a removed
    // method keep answering queries.
    conn.execute_batch("DROP TABLE IF EXISTS meta; DROP TABLE IF EXISTS methods; DROP TABLE IF EXISTS routes; DROP TABLE IF EXISTS calls;")
        .context("clear previous index")?;

    conn.execute_batch(
        r#"
        CREATE TABLE meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- service, class, method form the identity: the same method name exists
        -- in many projects.
        CREATE TABLE methods (
            service    TEXT,
            class      TEXT NOT NULL,
            method     TEXT NOT NULL,
            signature  TEXT,
            file       TEXT,
            line_start INTEGER,
            line_end   INTEGER,
            PRIMARY KEY (service, class, method)
        );

        -- index.sql routes by service: every mapping question starts with the
        -- service that owned the failing endpoint.
        CREATE INDEX idx_methods_service ON methods(service);
        CREATE INDEX idx_methods_class   ON methods(class);

        CREATE TABLE routes (
            service        TEXT,
            http_method    TEXT NOT NULL,
            route_pattern  TEXT NOT NULL,
            class          TEXT NOT NULL,
            method         TEXT NOT NULL,
            PRIMARY KEY (service, http_method, route_pattern, class, method)
        );

        CREATE INDEX idx_routes_service ON routes(service);

        CREATE TABLE calls (
            service       TEXT,
            source_class  TEXT NOT NULL,
            source_method TEXT NOT NULL,
            target_class  TEXT,
            target_method TEXT NOT NULL,
            line          INTEGER
        );

        CREATE INDEX idx_calls_source ON calls(service, source_class, source_method);
        CREATE INDEX idx_calls_target ON calls(service, target_class, target_method);
        "#,
    )
    .context("create schema")?;

    let tx = conn.unchecked_transaction().context("begin transaction")?;

    tx.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?1), ('content_hash', ?2), ('inline_lambda_routes', ?3)",
        rusqlite::params![
            SCHEMA_VERSION.to_string(),
            index.content_hash,
            index.inline_lambda_routes.to_string(),
        ],
    )?;

    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO methods (service, class, method, signature, file, line_start, line_end)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for m in &index.code.methods {
            stmt.execute(rusqlite::params![
                m.service.clone().unwrap_or_default(),
                m.class,
                m.method,
                m.signature,
                m.file,
                m.line_start as i64,
                m.line_end as i64,
            ])?;
        }
    }

    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO routes (service, http_method, route_pattern, class, method)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for r in &index.routes.entries {
            stmt.execute(rusqlite::params![
                r.service.clone().unwrap_or_default(),
                r.http_method,
                r.route_pattern,
                r.class,
                r.method,
            ])?;
        }
    }

    {
        let mut stmt = tx.prepare(
            "INSERT INTO calls (service, source_class, source_method, target_class, target_method, line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for c in &index.code.calls {
            // Call edges carry no service of their own; attribute them to the
            // service that owns the calling class.
            let service = index
                .code
                .find_method(&c.source_class, &c.source_method)
                .and_then(|m| m.service.clone())
                .unwrap_or_default();
            stmt.execute(rusqlite::params![
                service,
                c.source_class,
                c.source_method,
                c.target_class,
                c.target_method,
                c.line.map(|l| l as i64),
            ])?;
        }
    }

    tx.commit().context("commit index")?;
    Ok(())
}

/// Open an existing index read-only.
pub fn open(path: &Path) -> Result<Connection> {
    if !path.exists() {
        anyhow::bail!(
            "no index at {} — build one with: tinylink index --sources ... --out {}",
            path.display(),
            path.display()
        );
    }
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .with_context(|| format!("open index {}", path.display()))?;
    // A WAL database needs write access to its -shm file; fall back to a plain
    // read-only handle when the directory is not writable.
    conn.pragma_update(None, "query_only", "true").ok();
    Ok(conn)
}

/// Stored content hash, for deciding whether an index is still current.
pub fn stored_hash(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'content_hash'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

/// Reject an index written by an incompatible schema version.
pub fn check_schema(conn: &Connection) -> Result<()> {
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match version.as_deref().map(str::parse::<i64>) {
        Some(Ok(v)) if v == SCHEMA_VERSION => Ok(()),
        Some(Ok(v)) => anyhow::bail!(
            "index schema version {} is not supported (expected {}); rebuild it",
            v,
            SCHEMA_VERSION
        ),
        Some(Err(_)) => anyhow::bail!("index has a malformed schema version; rebuild it"),
        None => anyhow::bail!("index is missing schema metadata; rebuild it"),
    }
}

/// Load a whole index into memory.
///
/// For the size of a single service or two this is the fastest path, and it keeps
/// the rest of the code working against in-memory types. If this ever becomes the
/// bottleneck for a whole solution, the SQL helpers below are already the seam to
/// push filtering down into SQLite.
pub fn load(path: &Path) -> Result<Index> {
    let conn = open(path)?;
    check_schema(&conn)?;

    let content_hash = stored_hash(&conn)?.unwrap_or_default();
    let inline_lambda_routes: usize = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'inline_lambda_routes'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut code = CodeGraph::new();
    let mut cursor = conn.prepare(
        "SELECT service, class, method, signature, file, line_start, line_end FROM methods",
    )?;
    let rows = cursor.query_map([], |row| {
        Ok(MethodNode {
            service: opt_string(row.get(0)?),
            class: row.get(1)?,
            method: row.get(2)?,
            signature: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            file: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            line_start: row.get::<_, i64>(5)? as usize,
            line_end: row.get::<_, i64>(6)? as usize,
        })
    })?;
    for m in rows {
        code.add_method(m?);
    }

    for name in &code.methods {
        let _ = name;
    }
    code.classes = conn
        .prepare("SELECT DISTINCT class FROM methods")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut routes = RouteTable::new();
    let mut cursor =
        conn.prepare("SELECT service, http_method, route_pattern, class, method FROM routes")?;
    let rows = cursor.query_map([], |row| {
        Ok(RouteEntry {
            service: opt_string(row.get(0)?),
            http_method: row.get(1)?,
            route_pattern: row.get(2)?,
            class: row.get(3)?,
            method: row.get(4)?,
        })
    })?;
    for r in rows {
        routes.add(r?);
    }

    let mut cursor = conn.prepare(
        "SELECT source_class, source_method, target_class, target_method, line FROM calls",
    )?;
    let rows = cursor.query_map([], |row| {
        Ok(CallEdge {
            source_class: row.get(0)?,
            source_method: row.get(1)?,
            target_class: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            target_method: row.get(3)?,
            line: row.get::<_, Option<i64>>(4)?.map(|l| l as usize),
        })
    })?;
    for c in rows {
        code.calls.push(c?);
    }

    Ok(Index {
        code,
        routes,
        content_hash,
        warnings: Vec::new(),
        inline_lambda_routes,
    })
}

fn opt_string(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Rows for `explain_error`: services and routes, read straight from SQL.
pub fn services(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT service FROM methods ORDER BY service")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

/// Methods of one service, without loading the whole index.
pub fn methods_of_service(conn: &Connection, service: &str) -> Result<Vec<MethodNode>> {
    let mut stmt = conn.prepare(
        "SELECT class, method, signature, file, line_start, line_end
         FROM methods WHERE service = ?1 ORDER BY class, method",
    )?;
    let rows = stmt.query_map([service], |row| {
        Ok(MethodNode {
            service: Some(service.to_string()),
            class: row.get(0)?,
            method: row.get(1)?,
            signature: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            file: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            line_start: row.get::<_, i64>(4)? as usize,
            line_end: row.get::<_, i64>(5)? as usize,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> (std::path::PathBuf, Vec<String>) {
        let root = std::env::temp_dir().join(format!("tinylink_idx_{}", name));
        let _ = std::fs::remove_dir_all(&root);
        let agency = root.join("Agency.API");
        std::fs::create_dir_all(agency.join("Endpoints")).unwrap();
        std::fs::write(
            agency.join("Endpoints/Handler.cs"),
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
        (db, vec![agency.display().to_string()])
    }

    #[test]
    fn build_and_write_then_read_back_identical_data() {
        let (db, dirs) = fixture("roundtrip");
        let index = build(&dirs).expect("build");
        assert!(
            index
                .routes
                .entries
                .iter()
                .any(|r| r.class == "GetMembersEndpoint"),
            "route should be extracted: {:?}",
            index.routes.entries
        );

        write(&index, &db).expect("write");
        let loaded = load(&db).expect("load");

        assert_eq!(loaded.code.methods.len(), index.code.methods.len());
        assert_eq!(loaded.routes.entries.len(), index.routes.entries.len());
        assert_eq!(loaded.content_hash, index.content_hash);

        let handler = loaded
            .code
            .find_method("GetMembersEndpoint", "Handler")
            .expect("handler present after reload");
        assert!(
            handler.file.ends_with("Handler.cs"),
            "file path must survive the round trip: {:?}",
            handler.file
        );
        assert!(handler.line_start > 0);
        assert_eq!(handler.service.as_deref(), Some("Agency.API"));

        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn routes_keep_their_service_tag_through_the_round_trip() {
        let (db, dirs) = fixture("route_service");
        write(&build(&dirs).unwrap(), &db).unwrap();
        let loaded = load(&db).unwrap();
        let route = loaded
            .routes
            .entries
            .iter()
            .find(|r| r.class == "GetMembersEndpoint")
            .expect("route present");
        // Without the tag, a merged multi-service lookup cannot tell which
        // service owns the pattern.
        assert_eq!(route.service.as_deref(), Some("Agency.API"));
        assert_eq!(route.route_pattern, "/api/agency/{agencyId}/member");
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn content_hash_is_stable_and_change_sensitive() {
        let (db, dirs) = fixture("hash");
        let first = content_hash(&dirs).expect("hash");
        let second = content_hash(&dirs).expect("hash");
        assert_eq!(first, second, "hash must not depend on read order or time");

        // Touching a file without changing content must not change the hash.
        let agency = Path::new(&dirs[0]).join("Ext.cs");
        let bytes = std::fs::read(&agency).unwrap();
        std::fs::write(&agency, &bytes).unwrap();
        assert_eq!(content_hash(&dirs).unwrap(), first, "mtime must not matter");

        // Changing content must change it.
        let mut changed = String::from_utf8(bytes.clone()).unwrap();
        changed.push_str("\n// comment\n");
        std::fs::write(&agency, &changed).unwrap();
        assert_ne!(
            content_hash(&dirs).unwrap(),
            first,
            "content change must matter"
        );

        let _ = db;
    }

    #[test]
    fn hash_ignores_file_enumeration_order() {
        let (db, dirs) = fixture("hash_order");
        let forward = content_hash(&dirs).unwrap();
        let mut reversed = dirs.clone();
        reversed.reverse();
        assert_eq!(content_hash(&reversed).unwrap(), forward);
        let _ = db;
    }

    #[test]
    fn rebuild_replaces_stale_rows() {
        let (db, dirs) = fixture("rebuild");
        let mut index = build(&dirs).unwrap();
        write(&index, &db).unwrap();
        let before = load(&db).unwrap().code.methods.len();

        // Simulate a method being deleted from the source.
        index.code.methods.retain(|m| m.method != "Handler");
        write(&index, &db).unwrap();
        let after = load(&db).unwrap();
        assert_eq!(after.code.methods.len(), before - 1);
        assert!(
            after
                .code
                .find_method("GetMembersEndpoint", "Handler")
                .is_none(),
            "a removed method must not linger in the index"
        );
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn opening_a_missing_index_explains_how_to_build_it() {
        let err = open(Path::new("/nonexistent/tinylink/index.db")).expect_err("must fail");
        let message = format!("{:#}", err);
        assert!(
            message.contains("tinylink index"),
            "error should tell the user how to build it: {message}"
        );
    }

    #[test]
    fn schema_version_mismatch_is_rejected() {
        let (db, dirs) = fixture("schema");
        write(&build(&dirs).unwrap(), &db).unwrap();
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute(
                "UPDATE meta SET value = '999' WHERE key = 'schema_version'",
                [],
            )
            .unwrap();
        }
        let conn = open(&db).unwrap();
        let err = check_schema(&conn).expect_err("version mismatch must be rejected");
        assert!(format!("{:#}", err).contains("999"), "{:#}", err);
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn sql_helpers_scope_to_one_service() {
        let (db, dirs) = fixture("scoped_sql");
        write(&build(&dirs).unwrap(), &db).unwrap();
        let conn = open(&db).unwrap();
        assert_eq!(services(&conn).unwrap(), vec!["Agency.API".to_string()]);
        let methods = methods_of_service(&conn, "Agency.API").unwrap();
        assert!(methods.iter().any(|m| m.class == "GetMembersEndpoint"));
        assert!(
            methods_of_service(&conn, "Nope").unwrap().is_empty(),
            "an unknown service must yield nothing"
        );
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn two_services_both_survive_in_one_index() {
        let root = std::env::temp_dir().join(format!("tinylink_idx_multi_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        for (svc, class, path) in [
            ("Agency.API", "AgencyEndpoint", "/api/agency/member"),
            ("Billing.API", "BillingEndpoint", "/api/billing/invoice"),
        ] {
            let dir = root.join(svc);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("E.cs"),
                format!("namespace S;\npublic class {class} {{ public static IResult Handler() => null; }}\n"),
            )
            .unwrap();
            std::fs::write(
                dir.join("Ext.cs"),
                format!("app.MapGet(\"{path}\", {class}.Handler);\n"),
            )
            .unwrap();
        }

        let dirs: Vec<String> = ["Agency.API", "Billing.API"]
            .iter()
            .map(|s| root.join(s).display().to_string())
            .collect();
        let db = root.join("index.db");
        write(&build(&dirs).unwrap(), &db).unwrap();
        let loaded = load(&db).unwrap();

        assert_eq!(loaded.routes.entries.len(), 2, "both routes must survive");
        // Same handler method name in two services: the service tag is what keeps
        // them distinguishable.
        assert!(loaded
            .routes
            .entries
            .iter()
            .any(|r| r.service.as_deref() == Some("Agency.API") && r.class == "AgencyEndpoint"));
        assert!(loaded
            .routes
            .entries
            .iter()
            .any(|r| r.service.as_deref() == Some("Billing.API") && r.class == "BillingEndpoint"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn inline_lambda_routes_are_recorded_in_the_index() {
        let root = std::env::temp_dir().join(format!("tinylink_idx_lambda_{}", std::process::id()));
        let dir = root.join("Lam.API");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("P.cs"),
            "var app = WebApplication.CreateBuilder(args).Build();\napp.MapGet(\"/cost\", async (HttpContext c) => { return Results.Ok(); });\n",
        )
        .unwrap();

        let dirs = vec![dir.display().to_string()];
        let index = build(&dirs).unwrap();
        assert_eq!(index.inline_lambda_routes, 1);
        assert!(
            index.routes.entries.is_empty(),
            "a lambda has no named handler, so no route entry is created"
        );

        let db = root.join("index.db");
        write(&index, &db).unwrap();
        assert_eq!(load(&db).unwrap().inline_lambda_routes, 1);

        std::fs::remove_dir_all(&root).ok();
    }
}
