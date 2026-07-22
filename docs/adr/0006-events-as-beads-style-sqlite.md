# ADR 0006 — Event log as beads-style SQLite table with BLAKE3 dedup

- Status: Accepted
- Date: 2026-07-22
- Branch: `split-memory`

## Context

The "Ultra" vision requires a complete log of "what happened when, by
which agent" — the beads-style event stream. Synapse-memory v1 has
`query_logs` (search queries only) and `docs` (content), but no table
for agent actions like tool calls, file writes, decisions, messages.

## Decision

Add a single `synapse_events` table:

```sql
CREATE TABLE synapse_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          INTEGER NOT NULL,
    session_id  TEXT,
    agent       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    uri         TEXT,
    content     TEXT,
    content_zst BLOB,
    blake3      BLOB NOT NULL,
    meta        TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE SET NULL
);
```

- **`kind`** is a TEXT column (not an enum) — accepts any string. The
  Rust `EventKind` enum maps known kinds; unknown kinds are stored as-is.
- **`blake3`** is the dedup key: `blake3(session_id || agent || kind || uri || content)`.
  Identical events within the same session are collapsed to one row.
- **`content_zst`** holds zstd-compressed content for rows > 1 KB. The
  `zstd-compress` feature is on by default; disabling it stores all
  content as TEXT.
- **Indexes** on `(ts)`, `(agent, ts)`, `(session_id)`, `(kind, ts)`,
  `(uri)`, `(blake3)` — cover the common query patterns.

A separate `decisions` table holds the subset of events that represent
agent decisions (with `source_uri` and `target_uri`). A trigger
(`trg_decision_to_graph`) auto-populates `graph_nodes` + `graph_edges`
on decision insert, so the graph stays in sync with the event log.

A `token_cost` table holds per-call token usage for cost analytics.

## Consequences

- **Storage cost:** ~200 bytes per event uncompressed, ~50 bytes with
  zstd for typical content. 100k events ≈ 5 MB — fits comfortably in
  SQLite.
- **Query speed:** all common filters (agent, session, kind, uri, ts
  range) hit an index. Full-table scans are avoidable.
- **Dedup is content-based, not time-based** — two identical events in
  the same session collapse even if they happen minutes apart. This is
  correct for agent memory (repeated tool calls with same args = one
  memory).
- **No dependency on beads** — the format is beads-*style*, not beads
  itself. We reuse the idea (event stream as memory) without the
  runtime.

## Alternatives Rejected

- **Append-only JSONL file** — no indexes, no SQL, no dedup. Rejected.
- **DuckDB for events** — would require DuckDB at runtime. Rejected for
  default install; DuckLake archive is optional (ADR 0007).
- **Use the existing `query_logs` table** — wrong shape (search-only).
