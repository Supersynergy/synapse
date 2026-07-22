//! Graph-v2: SQLite-CTE-based graph traversal.
//!
//! Replaces the broken `synapse-graph` Datalog implementation (which falls
//! over above ~100 facts). All traversal is done via recursive SQL CTEs
//! backed by `graph_nodes` + `graph_edges`. The `why()` operator is a
//! backward chain (what caused this URI?); `graph_expand()` is a forward
//! chain (what does this URI lead to?).

use crate::UltraResult;
use rusqlite::{params, Connection};

/// A node in the graph.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub uri: String,
    pub kind: String,
    pub title: Option<String>,
    pub first_seen: i64,
    pub last_seen: i64,
}

/// An edge in the graph.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from_uri: String,
    pub to_uri: String,
    pub rel: String,
    pub weight: f64,
    pub ts: i64,
}

/// One step in a `why()` chain. `depth` = 0 is the starting URI.
#[derive(Debug, Clone)]
pub struct WhyStep {
    pub uri: String,
    pub kind: String,
    pub depth: i64,
    pub path: String,
}

/// Upsert a graph node. Updates `last_seen` if the node already exists.
pub fn upsert_node(
    conn: &Connection,
    uri: &str,
    kind: &str,
    title: Option<&str>,
    ts: i64,
) -> UltraResult<()> {
    conn.execute(
        "INSERT INTO graph_nodes (uri, kind, title, first_seen, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(uri) DO UPDATE SET last_seen = ?4, title = COALESCE(?3, title)",
        params![uri, kind, title, ts],
    )?;
    Ok(())
}

/// Insert or update an edge. On conflict (same from/to/rel), update weight + ts.
pub fn upsert_edge(
    conn: &Connection,
    from_uri: &str,
    to_uri: &str,
    rel: &str,
    weight: f64,
    ts: i64,
    session_id: Option<&str>,
    agent: Option<&str>,
) -> UltraResult<()> {
    conn.execute(
        "INSERT INTO graph_edges (from_uri, to_uri, rel, weight, ts, session_id, agent)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(from_uri, to_uri, rel) DO UPDATE SET weight = ?4, ts = ?5",
        params![from_uri, to_uri, rel, weight, ts, session_id, agent],
    )?;
    Ok(())
}

/// `why(uri, max_depth)` — backward chain: what caused this URI?
///
/// Returns steps ordered by depth ascending (0 = the starting node, 1 = direct
/// causes, etc.). Uses a parameterized recursive CTE anchored at `uri`.
pub fn why(conn: &Connection, uri: &str, max_depth: i64) -> UltraResult<Vec<WhyStep>> {
    let sql = r#"
WITH RECURSIVE chain(uri, kind, depth, path) AS (
    SELECT uri, kind, 0, uri FROM graph_nodes WHERE uri = ?1
    UNION ALL
    SELECT e.from_uri, n.kind, c.depth + 1, c.path || ' <- ' || e.from_uri
    FROM chain c
    JOIN graph_edges e ON e.to_uri = c.uri
    JOIN graph_nodes n ON n.uri = e.from_uri
    WHERE c.depth < ?2 AND e.rel IN ('caused', 'derived_from', 'depends_on')
)
SELECT uri, kind, depth, path FROM chain ORDER BY depth ASC, uri ASC
"#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![uri, max_depth], |row| {
        Ok(WhyStep {
            uri: row.get(0)?,
            kind: row.get(1)?,
            depth: row.get(2)?,
            path: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// `graph_expand(uri, max_depth)` — forward chain: what does this URI lead to?
pub fn graph_expand(conn: &Connection, uri: &str, max_depth: i64) -> UltraResult<Vec<WhyStep>> {
    let sql = r#"
WITH RECURSIVE expand(uri, kind, depth, path) AS (
    SELECT uri, kind, 0, uri FROM graph_nodes WHERE uri = ?1
    UNION ALL
    SELECT e.to_uri, n.kind, ex.depth + 1, ex.path || ' -> ' || e.to_uri
    FROM expand ex
    JOIN graph_edges e ON e.from_uri = ex.uri
    JOIN graph_nodes n ON n.uri = e.to_uri
    WHERE ex.depth < ?2
)
SELECT uri, kind, depth, path FROM expand ORDER BY depth ASC, uri ASC
"#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![uri, max_depth], |row| {
        Ok(WhyStep {
            uri: row.get(0)?,
            kind: row.get(1)?,
            depth: row.get(2)?,
            path: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Get a node by URI.
pub fn get_node(conn: &Connection, uri: &str) -> UltraResult<Option<GraphNode>> {
    let mut stmt = conn.prepare(
        "SELECT uri, kind, title, first_seen, last_seen FROM graph_nodes WHERE uri = ?1",
    )?;
    let mut rows = stmt.query(params![uri])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(GraphNode {
            uri: row.get(0)?,
            kind: row.get(1)?,
            title: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
        }));
    }
    Ok(None)
}

/// Get all edges from a URI.
pub fn edges_from(conn: &Connection, uri: &str) -> UltraResult<Vec<GraphEdge>> {
    let mut stmt = conn.prepare(
        "SELECT from_uri, to_uri, rel, weight, ts FROM graph_edges WHERE from_uri = ?1 ORDER BY ts",
    )?;
    let rows = stmt.query_map(params![uri], |row| {
        Ok(GraphEdge {
            from_uri: row.get(0)?,
            to_uri: row.get(1)?,
            rel: row.get(2)?,
            weight: row.get(3)?,
            ts: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Get all edges to a URI.
pub fn edges_to(conn: &Connection, uri: &str) -> UltraResult<Vec<GraphEdge>> {
    let mut stmt = conn.prepare(
        "SELECT from_uri, to_uri, rel, weight, ts FROM graph_edges WHERE to_uri = ?1 ORDER BY ts",
    )?;
    let rows = stmt.query_map(params![uri], |row| {
        Ok(GraphEdge {
            from_uri: row.get(0)?,
            to_uri: row.get(1)?,
            rel: row.get(2)?,
            weight: row.get(3)?,
            ts: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Count nodes and edges.
pub fn counts(conn: &Connection) -> UltraResult<(i64, i64)> {
    let nodes: i64 = conn.query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))?;
    let edges: i64 = conn.query_row("SELECT COUNT(*) FROM graph_edges", [], |row| row.get(0))?;
    Ok((nodes, edges))
}

/// Export the graph around a URI as Graphviz DOT format (for `synapse graph --dot`).
pub fn to_dot(conn: &Connection, uri: &str, max_depth: i64) -> UltraResult<String> {
    let steps = graph_expand(conn, uri, max_depth)?;
    let mut dot = String::from("digraph synapse {\n  rankdir=LR;\n  node [shape=box];\n");
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for s in &steps {
        if seen.insert(s.uri.clone()) {
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", s.uri, s.uri));
        }
    }
    // Add edges between consecutive steps in each path
    for s in &steps {
        let parts: Vec<&str> = s.path.split(" -> ").collect();
        for w in parts.windows(2) {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", w[0], w[1]));
        }
    }
    dot.push_str("}\n");
    Ok(dot)
}
