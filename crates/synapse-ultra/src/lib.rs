//! Synapse Ultra — lean extension layer for synapse-memory.
//!
//! Adds four capabilities on top of the existing SQLite brain.db:
//!   1. `synapse_events` — beads-style event log (what happened when, by whom)
//!   2. `graph_nodes` / `graph_edges` — graph-v2 as SQLite-CTE (replaces broken Datalog)
//!   3. `why()` / `graph_expand()` — recursive CTE views for decision-chain traversal
//!   4. Observe CLI — inspect / why / replay / cost / events / graph / lake
//!
//! Design goals (ADR 0004-0008):
//!   - No modification to existing synapse-core schema (additive only)
//!   - Idempotent migration (`synapse-ultra init` is safe to run repeatedly)
//!   - Single-file storage (same brain.db, or own ultra.db)
//!   - Production-ready: WAL, indexes, BLAKE3 dedup, zstd compression for large content
//!
//! # Example
//! ```no_run
//! use synapse_ultra::Ultra;
//! let ultra = Ultra::open("/path/to/brain.db")?;
//! ultra.migrate()?;
//! ultra.with_conn(|c| synapse_ultra::events::ingest_event_json(
//!     c,
//!     r#"{"agent":"claude","kind":"decision","uri":"file:foo.rs","content":"refactored"}"#,
//! ))?;
//! let chain = ultra.with_conn(|c| synapse_ultra::graph::why(c, "file:foo.rs", 5))?;
//! # Ok::<(), synapse_ultra::UltraError>(())
//! ```

pub mod error;
pub mod events;
pub mod graph;
pub mod lake;
pub mod observe;
pub mod schema;

pub use error::{UltraError, UltraResult};
pub use events::{Event, EventFilter, EventKind};
pub use graph::{GraphEdge, GraphNode, WhyStep};
pub use lake::LakeState;
pub use observe::{BrainStats, CostRow, ReplayEntry};

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The Synapse Ultra handle. Wraps a single SQLite connection in WAL mode.
///
/// Thread-safe via internal mutex. For higher concurrency, open multiple
/// `Ultra` handles to the same file (SQLite WAL supports concurrent readers).
pub struct Ultra {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Ultra {
    /// Open or create a Synapse Ultra database at `path`.
    ///
    /// If `path` points to an existing synapse-memory `brain.db`, the new
    /// tables are added alongside `docs`/`docs_fts`/`docs_vec` without
    /// modifying them. If `path` does not exist, a fresh database is created.
    pub fn open<P: AsRef<Path>>(path: P) -> UltraResult<Self> {
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000_i64)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> UltraResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    /// Path of the underlying database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run the idempotent schema migration. Safe to call repeatedly.
    ///
    /// Adds: `synapse_events`, `graph_nodes`, `graph_edges`, `decisions`,
    /// `sessions`, `token_cost`, and the `why_chain` / `graph_expand_views` SQL views.
    pub fn migrate(&self) -> UltraResult<()> {
        let conn = self.lock_conn();
        schema::migrate(&conn)?;
        Ok(())
    }

    /// Access the underlying connection (for advanced users / tests).
    ///
    /// Holds the internal mutex for the duration of the closure.
    pub fn with_conn<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Connection) -> R,
    {
        let conn = self.lock_conn();
        f(&conn)
    }

    /// Lock the internal mutex, recovering from poison so a prior thread
    /// panic does not cascade into a crash on the next call.
    fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}
