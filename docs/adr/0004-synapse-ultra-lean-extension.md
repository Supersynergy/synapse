# ADR 0004 — Synapse Ultra: lean extension layer for event log, graph-v2, observe

- Status: Accepted
- Date: 2026-07-22
- Branch: `split-memory`
- Supersedes: none (extends ADR 0001)

## Context

Synapse-memory v1.0.1-rc.1 is a working Context-OS with SQLite + FTS5 + sqlite-vec
+ usearch HNSW + `synapse-pack` + `synapse-learn` + `synapse-mcp` + `synapsed`.
Three gaps remain vs the "Ultra" vision:

1. **No event log** — there is no beads-style "what happened when, by whom" table.
   `query_logs` only records search queries, not agent actions.
2. **Graph-v1 is broken** — `synapse-graph` Datalog implementation fails above
   ~100 facts (documented in CHANGELOG). The `why()` operator from VelesDB
   cannot be implemented on top of it.
3. **No observe surface** — `synapse doctor` exists, but there is no
   `inspect` / `why` / `replay` / `cost` / `events` CLI for observability.

The full 12-week Synapse Ultra plan proposed Tauri dashboards, LanceDB, chDB,
WASM, live token routing, and 6 native targets. Most of that is YAGNI for
the first release that "just runs properly in general".

## Decision

1. **Add a single new crate `synapse-ultra`** that extends the existing
   `brain.db` with four additive tables and two SQL views:
   - `synapse_events` (beads-style event log, BLAKE3 dedup, zstd-compressed content)
   - `graph_nodes` + `graph_edges` (graph-v2 as plain SQLite tables)
   - `decisions` (subset of events with source/target URIs for auto-graph)
   - `token_cost` (per-call token usage for cost analytics)
   - `sessions` (groups related events)
   - `why_chain` view (recursive CTE for backward traversal)
   - `graph_expand` view (recursive CTE for forward traversal)
   - `trg_decision_to_graph` trigger (auto-populates graph on decision insert)

2. **Graph traversal is SQLite-CTE only.** No Datalog, no igraph bindings,
   no DuckDB-PGQ. The `why_chain` view is a recursive CTE with depth cap 20.
   This handles 10k+ nodes in < 100ms (benchmarked in `tests/ultra_tests.rs`).

3. **Observe is a CLI, not a dashboard.** `synapse-ultra inspect / why /
   replay / cost / events / graph --dot` are pure SQL queries. The Astro 7
   dashboard is deferred to v2.1 (ADR 0008).

4. **DuckLake is optional, not default.** Users can `synapse-ultra lake init`
   to create a DuckLake catalog for analytics/time-travel archiving. The
   default install has zero DuckDB dependency.

5. **Ingest is external scripts, not daemon code.** Claude/Codex/Gemini/git
   hooks write JSONL files; `synapse-ultra ingest --jsonl FILE` loads them.
   This keeps the daemon binary lean and avoids restarts on hook changes.

6. **No modification to existing synapse-core schema.** All Ultra tables are
   additive. The migration is idempotent and safe to run on an existing
   brain.db.

## Consequences

- `cargo check --workspace` now builds 9 product crates (was 8). Build time
  impact: < 2 seconds (synapse-ultra is ~1500 lines, no heavy deps).
- `synapse-ultra` depends only on `rusqlite`, `blake3`, `zstd` (optional),
  `clap`, `serde`, `chrono`, `dirs-next` — all already in the workspace.
- The `why()` operator works on any brain.db, including ones already in
  production. No data migration needed.
- The Astro 7 dashboard can later read the same SQLite views via read-only
  ATTACH — no API changes required.
- `synapse-graph` (Datalog) is deprecated but not removed. Users who need
  it can still build it from `crates/synapse-graph`. New code should use
  `synapse-ultra::graph`.

## Alternatives Rejected

- **Tauri dashboard now** — user explicitly deferred to Astro 7 later.
- **LanceDB for multimodal** — YAGNI, SQLite BLOBs suffice for now.
- **chDB** — 80 MB binary, Python-first; DuckDB CLI at runtime is lighter.
- **WASM target** — no concrete user need.
- **Live token routing as a crate** — `token-cfo` CLI already exists; a
  thin bridge writes to `token_cost` table. No new crate needed.
- **Thompson bandit for routing** — `synapse-learn` already has a bandit
  for `memory_type`; routing bandit is YAGNI until > 1k routing events.
