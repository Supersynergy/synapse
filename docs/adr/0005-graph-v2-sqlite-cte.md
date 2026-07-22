# ADR 0005 — Graph-v2 as SQLite recursive CTE (deprecate Datalog)

- Status: Accepted
- Date: 2026-07-22
- Branch: `split-memory`

## Context

`synapse-graph` ships a Datalog implementation that is documented as
"broken above ~100 facts" in the synapse CHANGELOG. The VelesDB `why()`
operator — backward chain "what caused this URI?" — cannot be built on
top of the broken Datalog engine.

Three options were considered:
1. SQLite recursive CTEs over `graph_nodes` + `graph_edges` tables
2. `igraph` Rust bindings (native graph library)
3. DuckDB-PGQ (graph query extension for DuckDB)

## Decision

**Use SQLite recursive CTEs.** Specifically:

- `graph_nodes(uri UNIQUE, kind, title, first_seen, last_seen, meta)`
- `graph_edges(from_uri, to_uri, rel, weight, ts, session_id, agent, meta)`
  with `UNIQUE(from_uri, to_uri, rel)` and `ON CONFLICT` weight update.
- `why_chain` view: recursive CTE, backward traversal, depth cap 20,
  filters to `rel IN ('caused', 'derived_from', 'depends_on')`.
- `graph_expand` view: recursive CTE, forward traversal, depth cap 20.

The `why()` function is a single SQL query against the `why_chain` view
filtered by the starting URI. No Datalog, no extra dependencies.

## Consequences

- **Performance:** 10k-node linear chain `why()` in < 100ms (benchmarked).
  Good enough for agent-memory scale (a single agent session rarely
  produces more than a few thousand decisions).
- **No new dependencies:** uses only `rusqlite` which is already in the
  workspace.
- **Portability:** works in any SQLite database, including WASM builds
  (via `rusqlite` + `sqlite3` compiled to WASM) if we ever need that.
- **Depth cap 20** prevents infinite recursion on cyclic graphs. Can be
  raised per-query if needed.
- **`synapse-graph` Datalog is deprecated** but not removed. Existing
  users keep working. New code uses `synapse-ultra::graph`.

## Alternatives Rejected

- **igraph bindings** — adds 5+ MB to binary, native C dependency, build
  complexity. YAGNI for agent-memory scale.
- **DuckDB-PGQ** — requires DuckDB at runtime, not in default install.
  Would couple graph queries to the optional DuckLake feature.
- **Keep Datalog, fix it** — Datalog is fundamentally hard to scale; the
  fix would be a rewrite. CTEs are simpler and already work.
