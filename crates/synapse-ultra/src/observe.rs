//! Observe: brain stats, replay, cost analytics.
//!
//! Pure SQL queries — no mutations. Used by the `synapse-ultra` CLI and
//! (eventually) by the Astro 7 dashboard.

use crate::UltraResult;
use rusqlite::{params, Connection};
use serde::Serialize;

/// High-level brain statistics (for `synapse-ultra inspect`).
#[derive(Debug, Clone, Serialize)]
pub struct BrainStats {
    pub docs: i64,
    pub events: i64,
    pub decisions: i64,
    pub graph_nodes: i64,
    pub graph_edges: i64,
    pub sessions: i64,
    pub token_cost_rows: i64,
    pub total_cost_usd: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub db_size_bytes: u64,
    pub ultra_schema_version: u32,
}

/// Compute brain stats.
pub fn brain_stats(conn: &Connection) -> UltraResult<BrainStats> {
    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or(0)
    }
    let docs = count(conn, "docs");
    let events = count(conn, "synapse_events");
    let decisions = count(conn, "decisions");
    let graph_nodes = count(conn, "graph_nodes");
    let graph_edges = count(conn, "graph_edges");
    let sessions = count(conn, "sessions");
    let token_cost_rows = count(conn, "token_cost");
    let (total_cost_usd, total_input_tokens, total_output_tokens) = conn
        .query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM token_cost",
            [],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .unwrap_or((0.0, 0, 0));
    let db_size_bytes = conn
        .query_row("PRAGMA page_count", [], |row| {
            row.get::<_, i64>(0)
        })
        .ok()
        .and_then(|pages| {
            conn.query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0))
                .ok()
                .map(|size| (pages * size) as u64)
        })
        .unwrap_or(0);
    let ultra_schema_version = crate::schema::schema_version(conn);
    Ok(BrainStats {
        docs,
        events,
        decisions,
        graph_nodes,
        graph_edges,
        sessions,
        token_cost_rows,
        total_cost_usd,
        total_input_tokens,
        total_output_tokens,
        db_size_bytes,
        ultra_schema_version,
    })
}

/// One entry in a session replay (for `synapse-ultra replay <session>`).
#[derive(Debug, Clone, Serialize)]
pub struct ReplayEntry {
    pub ts: i64,
    pub kind: String,
    pub uri: Option<String>,
    pub content: Option<String>,
}

/// Replay a session chronologically. Returns events ordered by ts ASC.
pub fn replay(conn: &Connection, session_id: &str, limit: i64) -> UltraResult<Vec<ReplayEntry>> {
    let mut stmt = conn.prepare(
        "SELECT ts, kind, uri, content
         FROM synapse_events
         WHERE session_id = ?1
         ORDER BY ts ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![session_id, limit], |row| {
        Ok(ReplayEntry {
            ts: row.get(0)?,
            kind: row.get(1)?,
            uri: row.get(2)?,
            content: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// One row in a cost report (for `synapse-ultra cost`).
#[derive(Debug, Clone, Serialize)]
pub struct CostRow {
    pub bucket: String,
    pub agent: String,
    pub model: String,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
}

/// Query token cost aggregated by (agent, model, day). `since_ts` filters rows.
pub fn cost_by_day(conn: &Connection, since_ts: i64) -> UltraResult<Vec<CostRow>> {
    let mut stmt = conn.prepare(
        "SELECT
            date(ts, 'unixepoch') AS bucket,
            agent,
            model,
            COUNT(*) AS calls,
            SUM(input_tokens) AS input_tokens,
            SUM(output_tokens) AS output_tokens,
            SUM(cost_usd) AS cost_usd
         FROM token_cost
         WHERE ts >= ?1
         GROUP BY bucket, agent, model
         ORDER BY bucket DESC, agent, model",
    )?;
    let rows = stmt.query_map(params![since_ts], |row| {
        Ok(CostRow {
            bucket: row.get(0)?,
            agent: row.get(1)?,
            model: row.get(2)?,
            calls: row.get(3)?,
            input_tokens: row.get(4)?,
            output_tokens: row.get(5)?,
            cost_usd: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Top agents by event count (for `synapse-ultra inspect`).
pub fn top_agents(conn: &Connection, limit: i64) -> UltraResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT agent, COUNT(*) AS c FROM synapse_events GROUP BY agent ORDER BY c DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Top event kinds by count.
pub fn top_kinds(conn: &Connection, limit: i64) -> UltraResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT kind, COUNT(*) AS c FROM synapse_events GROUP BY kind ORDER BY c DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
