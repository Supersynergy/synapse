# ADR 0008 — Observability via CLI now, Astro 7 dashboard later

- Status: Accepted
- Date: 2026-07-22
- Branch: `split-memory`

## Context

The "Ultra" vision includes "easy reinschauen" — easy observability into
what the agent memory contains. Two options were considered:
1. Tauri desktop dashboard now
2. CLI observe commands now, Astro 7 web dashboard later

The user explicitly deferred the dashboard to "später und eher Astro 7".

## Decision

**Ship CLI observe commands in v2.0.0. Defer the dashboard to v2.1
as an Astro 7 web app.**

CLI commands (in `synapse-ultra`):
- `synapse-ultra inspect` — brain stats (counts, sizes, top agents/kinds)
- `synapse-ultra why --uri URI` — decision-chain (backward graph traversal)
- `synapse-ultra graph --uri URI [--dot]` — forward graph traversal
- `synapse-ultra replay --session ID` — chronological event replay
- `synapse-ultra cost [--days N]` — token cost report by day/agent/model
- `synapse-ultra events [--agent X --kind Y --session S]` — event filter
- `synapse-ultra doctor` — health check

All commands are pure SQL queries — no mutations, no daemon. Output is
human-readable text (with `--json` planned for v2.1 dashboard feed).

## Consequences

- **v2.0.0 ships sooner** — no Tauri build pipeline, no webview2/WebKit
  dependency, no frontend toolchain in the release.
- **CLI is scriptable** — `synapse-ultra inspect | jq` works.
- **Astro 7 dashboard in v2.1** can read the same SQLite views via
  read-only ATTACH. No API changes required. The dashboard runs as a
  static site served by `synapsed --http-observe :7421` (new flag).
- **`--json` output** will be added in v2.0.1 if needed for the Astro 7
  bridge. For now, text output is enough for human inspection.

## Alternatives Rejected

- **Tauri dashboard now** — adds 30+ MB to release, native toolchain
  complexity, deferred by user.
- **No observe at all** — violates the "easy reinschauen" promise.
- **Web dashboard served by synapsed now** — would require HTTP server
  + auth + static asset pipeline. Defer to v2.1.
