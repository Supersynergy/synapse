# ADR 0007 — DuckLake as optional WARM archive, not default

- Status: Accepted
- Date: 2026-07-22
- Branch: `split-memory`

## Context

DuckLake v1.0 (released April 2026) is a lakehouse format that uses a
SQL catalog (SQLite, DuckDB, or PostgreSQL) for metadata of Parquet
files. It offers snapshots, time travel, ACID, and columnar analytics
over Parquet — ideal for archiving old `synapse_events` rows and running
analytics on them.

However, requiring DuckDB at runtime would violate the "single-file,
no-cloud, no-Docker" promise of synapse-memory. Most users will never
need analytics over > 1M events.

## Decision

**DuckLake is optional, not default.** The `synapse-ultra lake` CLI
subcommands require `duckdb` CLI in PATH; if missing, they error with a
helpful install message. The default `synapse-ultra` binary has zero
DuckDB dependency.

Operations:
- `synapse-ultra lake init --catalog PATH` — creates the DuckLake
  catalog (a SQLite file) + data directory. Idempotent.
- `synapse-ultra lake archive --older-than N` — moves events older
  than N days from `brain.db` to Parquet in the DuckLake table.
- `synapse-ultra lake analytics` — starts an interactive DuckDB shell
  with both `brain.db` (read-only) and the DuckLake catalog attached.

## Consequences

- **Default install stays lean** — `brew install duckdb` is opt-in.
- **Users who want analytics get them** — DuckLake gives time travel,
  snapshots, and columnar scans over archived events.
- **The `token_cost` table stays in `brain.db`** — cost analytics for
  the last 90 days is always available without DuckLake. Only long-term
  archiving needs it.
- **No data lock-in** — Parquet is an open format; users can read
  archived events with DuckDB, Polars, Pandas, or anything that reads
  Parquet.

## Alternatives Rejected

- **DuckDB as the primary storage** — DuckDB vss HNSW has known
  persistence issues (WAL recovery crashes). Not production-ready for
  mutable primary storage.
- **LanceDB for archive** — 11k stars, good product, but adds a native
  dependency. Parquet via DuckLake is more portable.
- **chDB** — 80 MB binary, Python-first. DuckDB CLI is lighter and
  already what users would install for DuckLake.
