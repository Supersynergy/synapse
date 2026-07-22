# ADR 0001 — Context-OS product boundary & layered workspace

- Status: Accepted
- Date: 2026-06-02
- Branch: `split-memory`

## Context

Synapse grew to 54 crates — an "everything store" (market data, OLAP, TSDB,
Raft, MySQL/PG wire protocols, multimodal) on top of the actual memory engine.
The SPEC is explicit: the product is a **local-first Context OS** whose default
workflow is six commands — `prime`, `context`, `remember`, `feedback`,
`fresh-context`, `doctor` — and *"vector/FTS/graph/database work is substrate,
not the default first-run product promise."*

The `split-memory` carve already cut the off-product crates (market/olap/tsdb/
raft/server) and vendored the heavy storage substrate into `vendor/synapse-db`.
What remained was still a flat `members = ["crates/*"]` glob that pulled
advanced-retrieval, multimodal, and binding crates into every default build.

## Decision

1. **Layered architecture, dependency direction inward only.**

   | Layer | Crates | Role |
   |---|---|---|
   | L0 substrate | `vendor/synapse-db` (core, kernel, engine, ann, fts, graph) | SQLite + vec + FTS + SIMD kernels |
   | L1 domain | `synapse-extract`, `synapse-rerank`, `synapse-learn`, `synapse-space`, `synapse-temporal` | chunking, rerank, bandit feedback, namespaces, temporal parsing |
   | L2 interfaces | `synapsed` (socket daemon), `synapse-cli` (`synx`), `synapse-mcp` (MCP bridge) | the product surface |

   `synapse-mcp` talks to `synapsed` over the unix socket and has **no**
   intra-workspace path deps — interfaces stay decoupled. Keep it that way.

2. **The default workspace is the product surface only.** Everything else is
   excluded from `members` and built on demand:
   - substrate: `vendor/synapse-db`
   - bindings: `synapse-py`, `synapse-js`
   - platform/GPU: `synapse-metal`, `synapse-embed-gpu`
   - advanced retrieval (research): `synapse-colbert`, `synapse-splade`, `synapse-fusion`
   - multimodal/media: `synapse-multimodal`, `synapse-media`

3. **Crates split on need, not speculation.** A new boundary becomes its own
   crate only when it needs an independent build, version, or feature gate —
   not because it "feels separate".

## Consequences

- `cargo check --workspace` / CI builds the 8 product crates, not 14 → faster,
  smaller blast radius, matches the SPEC's release claims.
- An excluded crate is not reachable via `-p` from the workspace root; build it
  from its own directory. Acceptable for experimental/opt-in code.
- When an experimental crate graduates to the product surface, move it out of
  `exclude` and record it here.
- No code is deleted — the carve is a membership/decision change, fully
  reversible via git.
