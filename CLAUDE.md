# Synapse-Memory — project CLAUDE.md

Local-first **Context OS** for AI agents: bounded, cited, freshness-aware context with feedback. One SQLite-backed local brain — no Docker, no cloud. CLI + daemon + MCP tooling. Core promise: **best context, not biggest context.**

- Crate: `synapse-core` (crates.io) · MIT · Repo: https://github.com/Supersynergy/synapse-memory
- Local: `~/BASE/projects/synapse-memory` · current branch: **`split-memory`** (WIP)
- Runtime socket: `/tmp/synapse.sock` :9477 (SQLite fallback `~/.synapse/brain.db`)

## Stack
Rust workspace (edition 2024). SimSIMD kernels (1-bit 71× / int8 46× / MRL-128 35× / f16 4×). MRL/f16/Hamming vectors. Public product and substrate members live in `crates/*`.

## Commands (just)
```bash
just check        # default gate (fmt + clippy + check)
just test         # cargo nextest
just ci           # check + test          ← full gate
just build
just check-layers # architecture/layer boundary check
```
Direct: `cargo clippy --all-targets --all-features -- -D warnings` · `cargo nextest run`.

## Quality / security
`deny.toml` (cargo-deny) · `audit.toml` (cargo-audit) · `clippy.toml`. Run before release. `ARCHITECTURE.md` = layer map; `CONTRIBUTING.md` = workflow.

## Lean-code notes
Correctness-critical surface (vectors, SQLite, parsers) stays in the visible workspace and in `cargo audit`; never hide source behind machine-local path dependencies. Keep maintained dependencies and apply `/leancode` before adding crates.

## Release flow
`CHANGELOG.md` newest-first (currently `Unreleased`). Merge `split-memory` → cut a tagged version before claiming release.

Inherits global rules `~/.claude/CLAUDE.md` + workspace `~/BASE/projects/CLAUDE.md`.
