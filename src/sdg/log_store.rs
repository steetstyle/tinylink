//! Persistent log store, queried per incident.
//!
//! # Why this is separate from the code index
//!
//! [`crate::link::index`] holds static code metadata and deliberately excludes
//! runtime data: it is rebuilt when the source changes and cached until then.
//! Logs are the opposite — they arrive continuously, are never rebuilt, and are
//! only interesting for the window around an incident. Putting them in the code
//! index would either invalidate a good index on every log line or quietly mix
//! two lifetimes that have nothing to do with each other.
//!
//! # Searching attribute values, not just the body
//!
//! The OpenTelemetry .NET exporter emits the *message template* as the body. A
//! real capture contains
//!
//! ```text
//! body:     Handler {HandlerName} failed: {Message}
//! attribute: HandlerName = GetAgencyMembers
//! attribute: Message      = no name for agency 9f1c
//! ```
//!
//! so a search over the body alone cannot find `no name for agency 9f1c` in the
//! log that contains it — which is exactly the string someone pastes in during an
//! incident. Every record therefore gets a `search_text` column holding the body
//! followed by its `key=value` pairs, and text search runs against that.
//!
//! This is a `LIKE` scan rather than an FTS index: log volumes here are per
//! incident window, not per year, and `LIKE` keeps the store a single file with no
//! external tokenizer to keep in step with the SDK.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::log::{LogRecord, Severity};
use super::log_otlp::{parse_json, parse_protobuf, LogBatch};

/// Bumped when the layout changes; a mismatch discards the store rather than
/// reading it with the wrong expectations.
const SCHEMA_VERSION: i64 = 1;

/// Where to keep the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogStorePath {
    /// A file on disk, reused across runs.
    File(PathBuf),
    /// A private temporary file, discarded when dropped. For a one-shot CLI call.
    Temporary(PathBuf),
    /// Purely in memory.
    Memory,
}

/// A log store.
#[derive(Debug)]
pub struct LogStore {
    conn: Connection,
    #[allow(dead_code)]
    path: LogStorePath,
}

/// What an ingest did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngestReport {
    /// Records written.
    pub inserted: usize,
    /// Files or payloads read.
    pub payloads: usize,
    /// Distinct trace ids among the inserted records.
    pub traces: usize,
    /// Distinct service names among the inserted records.
    pub services: usize,
    /// Records skipped, with the reason, rather than dropped silently.
    pub skipped: Vec<String>,
}

/// A record as stored, with the service it came from.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredLog {
    #[serde(flatten)]
    pub record: LogRecord,
    pub service: String,
}

impl LogStore {
    /// Open or create a store.
    pub fn open(path: LogStorePath) -> Result<Self> {
        let (conn, resolved) = match &path {
            LogStorePath::Memory => (Connection::open_in_memory()?, LogStorePath::Memory),
            LogStorePath::File(p) | LogStorePath::Temporary(p) => {
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent).ok();
                    }
                }
                let conn = Connection::open(p)
                    .with_context(|| format!("cannot open log store at {}", p.display()))?;
                (conn, path.clone())
            }
        };
        let store = Self {
            conn,
            path: resolved,
        };
        store.migrate()?;
        Ok(store)
    }

    /// A store in a fresh temporary file.
    ///
    /// Unique per call, not per process: tests run in parallel threads and two
    /// of them sharing one file means one deletes the database the other is
    /// querying. A counter rather than a random name keeps the path predictable
    /// enough to clean up after a crashed run.
    pub fn temporary(tag: &str) -> Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("tinylink_logs_{tag}_{}_{n}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        Self::open(LogStorePath::Temporary(path))
    }

    fn migrate(&self) -> Result<()> {
        // WAL lets a running server read while an ingest writes.
        // `pragma_update` rejects pragmas that answer with a row, and
        // journal_mode does, so it is set as a statement instead.
        self.conn
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;

        // The version can only be read once the table exists, so its existence is
        // what tells a fresh store from an existing one.
        let version: Option<i64> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()
            .unwrap_or(None);

        if version.is_some() && version != Some(SCHEMA_VERSION) {
            // A stale layout is worse than none: drop it rather than misread it.
            for table in ["logs", "meta"] {
                self.conn
                    .execute(&format!("DROP TABLE IF EXISTS {table}"), [])?;
            }
        }

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS logs (
                 id                INTEGER PRIMARY KEY,
                 observed_unix_nano INTEGER NOT NULL,
                 has_time          INTEGER NOT NULL,
                 service           TEXT NOT NULL,
                 trace_id          TEXT,
                 span_id           TEXT,
                 severity_number   INTEGER NOT NULL,
                 severity_text     TEXT NOT NULL,
                 body              TEXT NOT NULL,
                 attributes        TEXT NOT NULL,
                 attributes_text   TEXT NOT NULL,
                 search_text       TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_logs_trace   ON logs(trace_id);
             CREATE INDEX IF NOT EXISTS idx_logs_service ON logs(service, observed_unix_nano);
             CREATE INDEX IF NOT EXISTS idx_logs_severity ON logs(severity_number);",
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES('schema_version', ?1)",
            params![SCHEMA_VERSION],
        )?;
        Ok(())
    }

    /// How many records the store holds.
    pub fn len(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM logs", [], |r| r.get::<_, i64>(0))?
            as usize)
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Add one decoded batch.
    ///
    /// The service name is taken from the batch's resource attributes. A record
    /// with no resolvable service is still stored, under the empty name, because
    /// dropping it would make a count silently wrong; it is reported in
    /// [`IngestReport::skipped`] instead so the gap is visible.
    pub fn insert_batch(&mut self, batch: &LogBatch) -> Result<IngestReport> {
        let service = batch.service_name().unwrap_or_default().to_string();
        let mut report = IngestReport {
            payloads: 1,
            ..Default::default()
        };
        if service.is_empty() {
            report.skipped.push(
                "batch carries no service.name resource attribute; \
                 records stored with an empty service"
                    .to_string(),
            );
        }

        let tx = self.conn.transaction()?;
        let mut traces: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut services: std::collections::HashSet<String> = std::collections::HashSet::new();
        if !service.is_empty() {
            services.insert(service.clone());
        }

        {
            let mut stmt = tx.prepare(
                "INSERT INTO logs (
                     observed_unix_nano, has_time, service, trace_id, span_id,
                     severity_number, severity_text, body, attributes,
                     attributes_text, search_text
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for record in &batch.records {
                let observed = record.observed_time_unix_nano.max(record.time_unix_nano);
                let has_time = i64::from(observed != 0);
                let trace = record.trace_id().map(|s| s.to_string());
                let span = record.span_id().map(|s| s.to_string());
                let severity = record.severity();
                let attributes = serde_json::to_string(&record.attributes).unwrap_or_default();

                stmt.execute(params![
                    observed as i64,
                    has_time,
                    service,
                    trace,
                    span,
                    i64::from(severity.number()),
                    record.severity_text,
                    record.body,
                    attributes,
                    attributes_text(&record.attributes),
                    search_text(&record.body, &record.attributes),
                ])?;
                report.inserted += 1;
                if let Some(t) = trace {
                    traces.insert(t);
                }
            }
        }

        tx.commit()?;
        report.traces = traces.len();
        report.services = services.len();
        Ok(report)
    }

    /// Add raw OTLP bytes, in either wire format.
    pub fn insert_bytes(&mut self, bytes: &[u8]) -> Result<IngestReport> {
        let batch = super::log_otlp::parse_otlp_http(bytes, None)?;
        self.insert_batch(&batch)
    }

    /// Is this file part of the store itself?
    ///
    /// Pointing an ingest at the directory the store lives in is the obvious thing
    /// to type — a captures directory with the store written beside it — and it
    /// makes the store read its own database as a log payload. SQLite's `-wal` and
    /// `-shm` sidecars get picked up too, so the exclusion covers those as well.
    fn is_own_file(&self, path: &Path) -> bool {
        let store = match &self.path {
            LogStorePath::File(p) | LogStorePath::Temporary(p) => p,
            LogStorePath::Memory => return false,
        };
        let same = |a: &Path, b: &Path| -> bool {
            match (a.canonicalize(), b.canonicalize()) {
                (Ok(a), Ok(b)) => a == b,
                // Before the file exists there is nothing canonical to compare, so
                // fall back to the literal paths.
                _ => a == b,
            }
        };
        if same(path, store) {
            return true;
        }
        // SQLite names its sidecars by appending to the whole file name, so
        // `with_extension` is wrong here: it would turn logs.db into logs.wal.
        // The suffixes already carry the leading hyphen.
        ["-wal", "-shm", "-journal"]
            .iter()
            .any(|suffix| same(path, Path::new(&format!("{}{suffix}", store.display()))))
    }

    /// Read every payload under `path` into the store.
    ///
    /// A directory is walked one level deep and each file is treated as one
    /// payload. A single file is one payload. That distinction matters: OTLP
    /// protobuf carries no total length, so several exports concatenated into one
    /// stream cannot be split apart reliably, and guessing would silently lose
    /// records. Requiring one payload per file makes a short read visible instead.
    ///
    /// The store's own files are skipped, so ingesting the directory the store
    /// lives in does not make it read itself.
    pub fn insert_path(&mut self, path: &Path) -> Result<IngestReport> {
        let mut report = IngestReport::default();
        if path.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(path)
                .with_context(|| format!("cannot read {}", path.display()))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect();
            // Sorted so a repeat ingest reports the same ordering every time.
            files.sort();

            let mut skipped_own = 0usize;
            for file in files {
                if self.is_own_file(&file) {
                    skipped_own += 1;
                    continue;
                }
                match self.insert_file(&file) {
                    Ok(r) => merge(&mut report, r),
                    Err(e) => report.skipped.push(format!("{}: {e:#}", file.display())),
                }
            }
            if skipped_own > 0 {
                report.skipped.push(format!(
                    "skipped {skipped_own} file(s) belonging to the log store itself"
                ));
            }
        } else {
            if self.is_own_file(path) {
                anyhow::bail!(
                    "{} is the log store itself, not a payload file",
                    path.display()
                );
            }
            merge(&mut report, self.insert_file(path)?);
        }
        Ok(report)
    }

    fn insert_file(&mut self, file: &Path) -> Result<IngestReport> {
        let bytes =
            std::fs::read(file).with_context(|| format!("cannot read {}", file.display()))?;
        // Sniff rather than trust the extension: exporters disagree about which
        // one a `.bin` capture really is, and a wrong guess costs the whole file.
        let trimmed: &[u8] = {
            let start = bytes
                .iter()
                .position(|b| !b.is_ascii_whitespace())
                .unwrap_or(bytes.len());
            &bytes[start..]
        };
        // `{` or `[` means JSON, anything else protobuf. Checking the first
        // non-space byte beats trusting the extension: exporters disagree about
        // what a `.bin` capture really is, and a wrong guess costs the file.
        let batch = match trimmed.first() {
            Some(b'{') | Some(b'[') => parse_json(trimmed)?,
            _ => parse_protobuf(&bytes)?,
        };
        let mut report = self.insert_batch(&batch)?;
        if report.inserted == 0 {
            report
                .skipped
                .push(format!("{}: no log records in payload", file.display()));
        }
        Ok(report)
    }

    /// Run a query.
    pub fn query(&self, q: &LogQuery) -> Result<LogQueryResult> {
        let scanned = self.len()?;
        let (sql, mut binds) = q.to_sql();

        // Count first: a paged response that does not say how many matched reads
        // as a complete answer.
        let total_matched: usize = {
            let refs = borrow_params(&binds);
            self.conn.query_row(
                &format!("SELECT COUNT(*) FROM ({sql})"),
                refs.as_slice(),
                |r| r.get::<_, i64>(0),
            )? as usize
        };

        let mut window_sql = sql;
        window_sql.push_str(" ORDER BY observed_unix_nano DESC, id DESC LIMIT ? OFFSET ?");
        binds.push(Box::new(i64::try_from(q.limit).unwrap_or(i64::MAX)));
        binds.push(Box::new(i64::try_from(q.offset).unwrap_or(i64::MAX)));

        let refs = borrow_params(&binds);
        let mut stmt = self.conn.prepare(&window_sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            // Column order follows the SELECT built in LogQuery::to_sql:
            // observed_unix_nano, id, service, trace_id, span_id,
            // severity_number, severity_text, body, attributes.
            let observed: i64 = row.get(0)?;
            let service: String = row.get(2)?;
            let attributes: String = row.get(8)?;
            Ok(StoredLog {
                service,
                record: LogRecord {
                    body: row.get(7)?,
                    severity_number: row.get::<_, i64>(5)? as u32,
                    severity_text: row.get(6)?,
                    attributes: serde_json::from_str(&attributes)
                        .unwrap_or_else(|_| BTreeMap::new()),
                    time_unix_nano: observed.max(0) as u64,
                    observed_time_unix_nano: observed.max(0) as u64,
                    trace_id: row.get::<_, Option<String>>(3)?,
                    span_id: row.get::<_, Option<String>>(4)?,
                },
            })
        })?;

        let mut matches = Vec::new();
        for row in rows {
            matches.push(row?);
        }

        let window_len = matches.len();
        Ok(LogQueryResult {
            matches,
            total_matched,
            offset: q.offset,
            scanned,
            untimed: if q.timed_window() {
                self.untimed_count()?
            } else {
                0
            },
            untraceable: if q.trace_id.is_some() {
                self.untraceable_count()?
            } else {
                0
            },
            truncated: q.offset + window_len < total_matched,
        })
    }

    /// Records that carry no timestamp, so a time-window query cannot include them.
    ///
    /// Reported rather than hidden: a log that a time filter silently dropped is
    /// indistinguishable from a log that was never emitted.
    fn untimed_count(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM logs WHERE has_time = 0", [], |r| {
                r.get::<_, i64>(0)
            })? as usize)
    }

    fn untraceable_count(&self) -> Result<usize> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM logs WHERE trace_id IS NULL OR trace_id = ''",
            [],
            |r| r.get::<_, i64>(0),
        )? as usize)
    }

    /// Distinct service names present.
    pub fn services(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT service FROM logs WHERE service != '' ORDER BY service")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// Remove every record.
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM logs", [])?;
        Ok(())
    }
}

/// Owned binds, borrowed for the call.
///
/// `rusqlite` takes `&[&dyn ToSql]`, and the owned values have to outlive the
/// statement, so they are collected into one vec that does.
fn borrow_params(binds: &[Box<dyn rusqlite::ToSql>]) -> Vec<&dyn rusqlite::ToSql> {
    binds.iter().map(|b| b.as_ref()).collect()
}

fn merge(into: &mut IngestReport, from: IngestReport) {
    into.inserted += from.inserted;
    into.payloads += from.payloads;
    into.skipped.extend(from.skipped);
    // Distinctness is only exact per ingest; the report is a summary, so a sum
    // across payloads would overstate it. Keep the largest single payload's count
    // and say so in the field name instead.
    into.traces = into.traces.max(from.traces);
    into.services = into.services.max(from.services);
}

/// The text a free-text search runs against: the body, then every attribute.
fn search_text(body: &str, attributes: &BTreeMap<String, String>) -> String {
    let mut out = body.to_string();
    out.push('\n');
    out.push_str(&attributes_text(attributes));
    out
}

/// Every attribute rendered as `key=value`, one per line.
///
/// Kept in its own column rather than sliced out of `search_text` because
/// `instr` reports only the *first* match: a token appearing in both the body and
/// an attribute would hide the attribute occurrence, and an attributes-only search
/// would find nothing.
fn attributes_text(attributes: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(attributes.len() * 24);
    for (k, v) in attributes {
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    out
}

/// A log query. Every filter is independent and optional.
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    /// Match this trace exactly.
    pub trace_id: Option<String>,
    /// Match this service exactly.
    pub service: Option<String>,
    /// Minimum severity. Records whose severity cannot be determined never match.
    pub min_severity: Option<Severity>,
    /// Case-insensitive substring, matched against the body and attribute values.
    pub contains: Option<String>,
    /// Restrict `contains` to attribute values rather than searching everything.
    pub attributes_only: bool,
    /// Restrict to records having an attribute whose key contains this.
    pub attribute_key: Option<String>,
    /// Inclusive lower bound on the timestamp, in Unix nanoseconds.
    pub since_unix_nano: Option<u64>,
    /// Exclusive upper bound on the timestamp, in Unix nanoseconds.
    pub until_unix_nano: Option<u64>,
    /// Only records that carry a trace id.
    pub only_traceable: bool,
    pub limit: usize,
    pub offset: usize,
}

impl LogQuery {
    /// Does this query restrict time?
    pub fn timed_window(&self) -> bool {
        self.since_unix_nano.is_some() || self.until_unix_nano.is_some()
    }

    /// The `SELECT` plus a `WHERE`, and the binds that go with it.
    fn to_sql(&self) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut where_parts: Vec<&str> = Vec::new();

        if let Some(ref trace) = self.trace_id {
            // A trace id is case-insensitive hex in practice; compare folded so a
            // pasted uppercase id still finds the record.
            where_parts.push("LOWER(trace_id) = LOWER(?N)");
            binds.push(Box::new(trace.clone()));
        }
        if let Some(ref service) = self.service {
            where_parts.push("service = ?N");
            binds.push(Box::new(service.clone()));
        }
        if let Some(severity) = self.min_severity {
            where_parts.push("severity_number >= ?N");
            binds.push(Box::from(i64::from(severity.number())));
        }
        if let Some(ref needle) = self.contains {
            if self.attributes_only {
                where_parts.push("INSTR(LOWER(attributes_text), LOWER(?N)) > 0");
            } else {
                where_parts.push("INSTR(LOWER(search_text), LOWER(?N)) > 0");
            }
            binds.push(Box::new(needle.clone()));
        }
        if let Some(ref key) = self.attribute_key {
            // The trailing `=` is what makes this a key match rather than a
            // substring match anywhere in the attribute list.
            where_parts.push("INSTR(LOWER(attributes_text), LOWER(?N)) > 0");
            binds.push(Box::new(format!("{key}=")));
        }
        if let Some(since) = self.since_unix_nano {
            where_parts.push("observed_unix_nano >= ?N");
            binds.push(Box::new(since as i64));
        }
        if let Some(until) = self.until_unix_nano {
            where_parts.push("observed_unix_nano < ?N");
            binds.push(Box::new(until as i64));
        }
        if self.only_traceable || self.trace_id.is_some() {
            where_parts.push("trace_id IS NOT NULL AND trace_id != ''");
        }

        const SELECT: &str = "SELECT observed_unix_nano, id, service, trace_id, span_id, \
                              severity_number, severity_text, body, attributes FROM logs";
        let sql = if where_parts.is_empty() {
            SELECT.to_string()
        } else {
            let clauses: Vec<String> = where_parts
                .iter()
                .enumerate()
                .map(|(i, clause)| clause.replace("?N", &format!("?{}", i + 1)))
                .collect();
            format!("{SELECT} WHERE {}", clauses.join(" AND "))
        };
        (sql, binds)
    }
}

/// What a query returned.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogQueryResult {
    pub matches: Vec<StoredLog>,
    /// Matches before `offset`/`limit` were applied.
    pub total_matched: usize,
    pub offset: usize,
    /// Whether more matches exist past this window.
    pub truncated: bool,
    /// Total records in the store, so a caller can tell an empty store from an
    /// over-narrow filter.
    pub scanned: usize,
    /// Records with no timestamp, which a time-window query cannot include.
    pub untimed: usize,
    /// Records with no trace id, which a trace query cannot include.
    pub untraceable: usize,
}

impl LogQueryResult {
    /// Distinct trace ids among the returned records.
    pub fn traces(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .matches
            .iter()
            .filter_map(|m| m.record.trace_id().map(|s| s.to_string()))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Cap a limit so a caller cannot ask for the whole store by accident.
pub const MAX_LIMIT: usize = 1000;

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::sdg::log::LogRecord;

    pub(super) fn record(body: &str, severity: Severity, trace: Option<&str>) -> LogRecord {
        let mut attributes = BTreeMap::new();
        if let Some(t) = trace {
            attributes.insert("TraceId".into(), t.into());
            attributes.insert("Service".into(), "agency".into());
        }
        LogRecord {
            body: body.into(),
            severity_number: severity.number(),
            severity_text: severity.as_str().into(),
            attributes,
            time_unix_nano: 1_700_000_000_000_000_000,
            observed_time_unix_nano: 1_700_000_000_000_000_000,
            trace_id: trace.map(|s| s.to_string()),
            span_id: None,
        }
    }

    fn batch(service: &str, records: Vec<LogRecord>) -> LogBatch {
        let mut resource = BTreeMap::new();
        resource.insert("service.name".into(), service.into());
        LogBatch {
            records,
            resource_attributes: resource,
        }
    }

    pub(super) fn store_with(records: Vec<LogRecord>) -> LogStore {
        let mut store = LogStore::temporary("unit").expect("store");
        store
            .insert_batch(&batch("Agency.API", records))
            .expect("insert");
        store
    }

    #[test]
    fn records_round_trip_through_the_store() {
        let store = store_with(vec![record("hello", Severity::Info, Some("t1"))]);
        assert_eq!(store.len().unwrap(), 1);

        let out = store
            .query(&LogQuery {
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 1);
        assert_eq!(out.matches[0].record.body, "hello");
        assert_eq!(out.matches[0].service, "Agency.API");
        assert_eq!(out.matches[0].record.trace_id(), Some("t1"));
    }

    /// The reason `search_text` exists. Without it, the value a developer pastes
    /// during an incident is invisible to search.
    #[test]
    fn text_search_finds_a_value_that_only_lives_in_an_attribute() {
        let mut r = record(
            "Handler {HandlerName} failed: {Message}",
            Severity::Error,
            Some("t1"),
        );
        r.attributes
            .insert("HandlerName".into(), "GetAgencyMembers".into());
        r.attributes
            .insert("Message".into(), "no name for agency 9f1c".into());
        let store = store_with(vec![r]);

        let by_value = store
            .query(&LogQuery {
                contains: Some("no name for agency".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(
            by_value.matches.len(),
            1,
            "attribute values must be searchable"
        );

        let by_template = store
            .query(&LogQuery {
                contains: Some("{HandlerName}".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(by_template.matches.len(), 1, "the body is searchable too");
    }

    /// `attributes_only` narrows to attributes, so a template token in the body
    /// does not match. It has to be able to tell the two apart, which is the whole
    /// reason the column holds both.
    #[test]
    fn attributes_only_excludes_matches_that_are_in_the_body() {
        let mut r = record("failed for {HandlerName}", Severity::Error, Some("t1"));
        r.attributes
            .insert("HandlerName".into(), "GetAgencyMembers".into());
        let store = store_with(vec![r]);

        let both = store
            .query(&LogQuery {
                contains: Some("HandlerName".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(both.matches.len(), 1);

        let attrs_only = store
            .query(&LogQuery {
                contains: Some("HandlerName".into()),
                attributes_only: true,
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(
            attrs_only.matches.len(),
            1,
            "HandlerName is also an attribute key here, so it should match"
        );

        let body_token = store
            .query(&LogQuery {
                contains: Some("{HandlerName}".into()),
                attributes_only: true,
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(
            body_token.matches.len(),
            0,
            "a body-only token must not match an attributes-only search"
        );
    }

    #[test]
    fn search_is_case_insensitive() {
        let store = store_with(vec![record(
            "Timeout on Upstream",
            Severity::Warn,
            Some("t1"),
        )]);
        let out = store
            .query(&LogQuery {
                contains: Some("upstream".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 1);
    }

    /// A severity filter must not promote a record whose severity could not be
    /// determined. `Unknown` numbers zero, so it sorts below every threshold.
    #[test]
    fn min_severity_excludes_records_of_unknown_severity() {
        let store = store_with(vec![
            record("fine", Severity::Info, Some("t1")),
            record("bad", Severity::Error, Some("t1")),
            record("mystery", Severity::Unknown, Some("t1")),
        ]);

        let errors = store
            .query(&LogQuery {
                min_severity: Some(Severity::Error),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(errors.matches.len(), 1);
        assert_eq!(errors.matches[0].record.body, "bad");
    }

    #[test]
    fn trace_filter_matches_case_insensitively() {
        let store = store_with(vec![record("x", Severity::Info, Some("ABCDEF0123"))]);
        let out = store
            .query(&LogQuery {
                trace_id: Some("abcdef0123".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 1);
    }

    /// A trace query has to admit that records without a trace id were excluded,
    /// otherwise "no logs found" is indistinguishable from "logs exist but none of
    /// them are linked".
    #[test]
    fn a_trace_query_reports_how_many_records_are_untraceable() {
        let store = store_with(vec![
            record("linked", Severity::Info, Some("t1")),
            record("floating", Severity::Info, None),
        ]);

        let out = store
            .query(&LogQuery {
                trace_id: Some("t1".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 1);
        assert_eq!(out.untraceable, 1);
        assert_eq!(out.scanned, 2, "the store holds more than matched");
    }

    #[test]
    fn a_time_window_reports_records_it_could_not_time() {
        let mut untimed = record("no clock", Severity::Info, Some("t1"));
        untimed.time_unix_nano = 0;
        untimed.observed_time_unix_nano = 0;
        let store = store_with(vec![untimed]);

        let out = store
            .query(&LogQuery {
                since_unix_nano: Some(1),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 0);
        assert_eq!(
            out.untimed, 1,
            "a record dropped for having no timestamp must be reported"
        );
    }

    #[test]
    fn paging_reports_the_pre_window_total() {
        let store = store_with(
            (0..7)
                .map(|i| record(&format!("line {i}"), Severity::Info, Some("t1")))
                .collect(),
        );

        let first = store
            .query(&LogQuery {
                contains: Some("line".into()),
                limit: 3,
                offset: 0,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(first.matches.len(), 3);
        assert_eq!(first.total_matched, 7);
        assert!(first.truncated);

        let last = store
            .query(&LogQuery {
                contains: Some("line".into()),
                limit: 3,
                offset: 6,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(last.matches.len(), 1);
        assert!(!last.truncated, "the final page must say there is no more");
    }

    #[test]
    fn service_filter_is_exact() {
        let mut store = LogStore::temporary("service").expect("store");
        store
            .insert_batch(&batch(
                "Agency.API",
                vec![record("a", Severity::Info, Some("t1"))],
            ))
            .unwrap();
        store
            .insert_batch(&batch(
                "Billing.API",
                vec![record("b", Severity::Info, Some("t2"))],
            ))
            .unwrap();

        assert_eq!(store.services().unwrap(), vec!["Agency.API", "Billing.API"]);
        let out = store
            .query(&LogQuery {
                service: Some("Billing.API".into()),
                limit: 10,
                ..Default::default()
            })
            .expect("query");
        assert_eq!(out.matches.len(), 1);
        assert_eq!(out.matches[0].record.body, "b");
    }

    /// A batch with no service.name is a configuration gap, not a reason to lose
    /// records. They are stored and the gap is reported.
    #[test]
    fn a_batch_without_a_service_name_is_reported_not_dropped() {
        let mut store = LogStore::temporary("nosvc").expect("store");
        let report = store
            .insert_batch(&LogBatch {
                records: vec![record("orphan", Severity::Info, Some("t1"))],
                resource_attributes: BTreeMap::new(),
            })
            .expect("insert");
        assert_eq!(report.inserted, 1);
        assert!(
            report.skipped.iter().any(|s| s.contains("service.name")),
            "{:?}",
            report.skipped
        );
        assert_eq!(store.len().unwrap(), 1);
    }

    #[test]
    fn the_store_persists_across_open() {
        let mut path = std::env::temp_dir();
        path.push(format!("tinylink_logs_persist_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));

        {
            let mut store = LogStore::open(LogStorePath::File(path.clone())).expect("open");
            store
                .insert_batch(&batch(
                    "Agency.API",
                    vec![record("kept", Severity::Info, Some("t1"))],
                ))
                .unwrap();
        }

        let reopened = LogStore::open(LogStorePath::File(path.clone())).expect("reopen");
        assert_eq!(reopened.len().unwrap(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn inserting_raw_otlp_bytes_works() {
        let payload = serde_json::json!({
            "resourceLogs": [{
                "resource": {"attributes": [
                    {"key": "service.name", "value": {"stringValue": "Agency.API"}}
                ]},
                "scopeLogs": [{"logRecords": [{
                    "severityText": "Error",
                    "severityNumber": 17,
                    "body": {"stringValue": "raw payload"},
                    "traceId": "t-raw"
                }]}]
            }]
        });
        let mut store = LogStore::temporary("raw").expect("store");
        let report = store
            .insert_bytes(&serde_json::to_vec(&payload).unwrap())
            .expect("insert");
        assert_eq!(report.inserted, 1, "{report:?}");
        assert_eq!(store.len().unwrap(), 1);
    }

    /// Ingesting the directory the store lives in must not make the store read
    /// itself.
    ///
    /// That is the obvious command to type — a captures directory with the store
    /// written beside it — and without the exclusion the store parses its own
    /// database as a log payload and ingests SQLite's `-wal`/`-shm` sidecars too.
    #[test]
    fn ingesting_the_stores_own_directory_skips_the_store() {
        let dir =
            std::env::temp_dir().join(format!("tinylink_logsel_{}_{}", std::process::id(), {
                use std::sync::atomic::{AtomicU64, Ordering};
                static N: AtomicU64 = AtomicU64::new(0);
                N.fetch_add(1, Ordering::Relaxed)
            }));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("capture.bin"),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/otlp-logs-capture.bin"
            )),
        )
        .unwrap();

        let store_path = dir.join("logs.db");
        let mut store = LogStore::open(LogStorePath::File(store_path.clone())).expect("open");
        // WAL mode creates the sidecars as soon as the store is written to, which
        // is what makes them visible to the directory walk.
        assert!(store.insert_batch(&batch("Agency.API", vec![])).is_ok());

        let report = store.insert_path(&dir).expect("ingest");
        assert_eq!(report.inserted, 8, "{report:?}");
        assert_eq!(
            report.payloads, 1,
            "only the capture is a payload: {report:?}"
        );
        assert!(
            report
                .skipped
                .iter()
                .any(|s| s.contains("belonging to the log store")),
            "the skip must be reported, not silent: {:?}",
            report.skipped
        );

        // Naming the store explicitly is a mistake worth catching too.
        let err = store.insert_path(&store_path).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("is the log store itself"),
            "{err:#}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file that turns out not to be a payload must be reported by name. Reading
    /// a directory of captures is an ordinary operation and one bad file should not
    /// lose the rest.
    #[test]
    fn an_unreadable_payload_is_reported_without_losing_the_others() {
        let dir = std::env::temp_dir().join(format!("tinylink_logdir_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("good.json"), br#"{"resourceLogs":[]}"#).unwrap();
        std::fs::write(dir.join("bad.json"), b"not json at all").unwrap();

        let mut store = LogStore::temporary("dir").expect("store");
        let report = store.insert_path(&dir).expect("ingest");

        assert!(
            report.skipped.iter().any(|s| s.contains("bad.json")),
            "{:?}",
            report.skipped
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ───────────────────────── JSON rendering ─────────────────────────

/// Render a query result as JSON, for both the MCP tool and the CLI.
///
/// Lives here rather than in the MCP tools because the log store is not an MCP
/// concern: `tinylink logs-search` must work without the `mcp` feature, and two
/// renderers would drift apart in exactly the fields that matter — the ones that
/// explain a short answer.
pub fn search_logs_json(store: &LogStore, query: &LogQuery) -> Result<serde_json::Value> {
    let result = store.query(query)?;

    let logs: Vec<serde_json::Value> = result
        .matches
        .iter()
        .map(|m| {
            let mut v = m.record.to_json();
            v["service"] = serde_json::Value::String(m.service.clone());
            v
        })
        .collect();

    Ok(serde_json::json!({
        "logs": logs,
        "count": result.matches.len(),
        "total_matched": result.total_matched,
        "offset": result.offset,
        "has_more": result.truncated,
        // Distinguishes "nothing matched" from "the store is empty" and from
        // "records exist but carry no timestamp, so a time window excluded them".
        "store_records": result.scanned,
        "excluded_untimed": result.untimed,
        "excluded_untraceable": result.untraceable,
        "traces_in_page": result.traces(),
        // Always present, so a wrong `service` guess is recoverable from the
        // response instead of needing a separate discovery call.
        "available_services": store.services()?,
    }))
}

impl LogQuery {
    /// Parse a severity name.
    ///
    /// An unrecognised name is an error rather than a default: silently treating
    /// `--min-severity critcal` as "no filter" would return the whole store and
    /// read as a confirmation that everything is fine.
    pub fn parse_severity(text: &str) -> Result<Severity> {
        let severity = Severity::from_number(0, text);
        if severity == Severity::Unknown {
            anyhow::bail!(
                "unknown severity {text:?}; use Trace, Debug, Info, Warn, Error or Fatal"
            );
        }
        Ok(severity)
    }

    /// Cap a caller-supplied limit.
    pub fn bounded_limit(limit: Option<usize>) -> anyhow::Result<usize> {
        let limit = limit.unwrap_or(MAX_LIMIT);
        if limit == 0 {
            anyhow::bail!("limit must be at least 1");
        }
        Ok(limit)
    }
}

/// Render a report as JSON, for both the MCP tool and the CLI.
pub fn ingest_logs_json(report: &IngestReport, store: &LogStore) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "records_inserted": report.inserted,
        "payloads_read": report.payloads,
        "distinct_traces": report.traces,
        "distinct_services": report.services,
        // Reported rather than swallowed: a missing service.name makes every later
        // service filter silently return nothing.
        "problems": report.skipped,
        "store_records": store.len()?,
        "services": store.services()?,
    }))
}

#[cfg(test)]
mod json_tests {
    use super::tests::{record, store_with};
    use super::*;

    fn seeded() -> LogStore {
        let store = store_with(vec![
            record("Handler {Name} failed", Severity::Error, Some("T1")),
            record("all quiet", Severity::Info, Some("T1")),
        ]);
        store
    }

    #[test]
    fn an_unrecognised_severity_is_refused() {
        assert!(LogQuery::parse_severity("catastrophic").is_err());
        // Spellings that do map are accepted, including the .NET LogLevel names.
        assert_eq!(LogQuery::parse_severity("Error").unwrap(), Severity::Error);
        assert_eq!(
            LogQuery::parse_severity("critical").unwrap(),
            Severity::Fatal
        );
        assert_eq!(LogQuery::parse_severity("WARNING").unwrap(), Severity::Warn);
    }

    #[test]
    fn a_zero_limit_is_refused_rather_than_returning_nothing() {
        assert!(LogQuery::bounded_limit(Some(0)).is_err());
        assert_eq!(LogQuery::bounded_limit(Some(5)).unwrap(), 5);
    }

    #[test]
    fn the_json_carries_the_fields_that_explain_a_short_answer() {
        let store = seeded();
        let json = search_logs_json(
            &store,
            &LogQuery {
                limit: 1,
                ..Default::default()
            },
        )
        .expect("json");

        assert_eq!(json["count"], 1);
        assert_eq!(json["total_matched"], 2, "the page is not the whole set");
        assert_eq!(json["has_more"], true);
        assert_eq!(json["store_records"], 2);
        assert_eq!(json["available_services"][0], "Agency.API");
        // Every entry names the service it came from; without it a log is not
        // attributable to anything.
        assert_eq!(json["logs"][0]["service"], "Agency.API");
    }

    #[test]
    fn the_ingest_json_reports_problems_alongside_the_counts() {
        let store = seeded();
        let report = IngestReport {
            inserted: 2,
            payloads: 1,
            traces: 1,
            services: 1,
            skipped: vec!["bad.json: not JSON".into()],
        };
        let json = ingest_logs_json(&report, &store).expect("json");
        assert_eq!(json["records_inserted"], 2);
        assert_eq!(json["problems"][0], "bad.json: not JSON");
        assert_eq!(json["store_records"], 2);
    }
}
