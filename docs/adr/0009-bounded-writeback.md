# ADR 0009: bounded write-back for high-frequency memory

Status: accepted, 2026-07-24.

## Context

Agent hooks can emit many small, delay-tolerant records. Starting a process,
opening SQLite, updating indexes, and syncing WAL for every event creates avoidable
latency, write amplification, lock pressure, and process churn.

Disabling SQLite durability or macOS safety mechanisms would reduce guarantees
for every write. The optimization belongs at the application boundary instead.

## Decision

Classify writes before persistence:

| Class | Examples | Path |
|---|---|---|
| durable now | user confirmation, checkpoint, security change, payment | direct `put`/`remember` |
| delayable | hook facts, session pointers, telemetry worth keeping | bounded RAM write-back |
| disposable | debug noise, duplicate notifications | aggregate or drop |

Delayable writes use one owner process, an in-memory ordered map, and three
simultaneous limits: maximum age, item count, and bytes. The first reached limit
flushes through `put-batch` in one SQLite transaction.

Required invariants:

1. Memory is bounded; producers cannot grow it without limit.
2. Every item has a stable idempotency key.
3. Backpressure spills or rejects explicitly; it never silently discards.
4. Disk spill is used only for overflow/failure and is owner-only plus fsynced.
5. A clean shutdown flushes. A failed flush spills before exit.
6. Retry has backoff, so a full or unavailable database does not cause a loop.
7. Metrics expose pending items/bytes, flush count, failures, spill bytes, and
   the configured durability window.
8. Critical callers bypass the buffer.

## Consequences

Normal bursts produce one process and one transaction instead of one of each per
event. WAL syncs and index maintenance are amortized across the batch. The
tradeoff is explicit: sudden power loss can lose the unflushed RAM window.

Failure spill adds disk I/O only when the normal path is already degraded. A
crash between downstream commit and local acknowledgement may replay a batch;
content-addressed deduplication makes that safe.

## Verification contract

- 20 independent enqueue operations become one downstream transaction.
- duplicate enqueue does not increase the pending set.
- buffer-offline enqueue spills and is replayed after restart.
- downstream failure spills before a forced hard crash.
- item, byte, frame, and time limits are exercised.
- direct durable writes remain unchanged.

SQLite WAL, sensible cache sizing, and OS page cache remain enabled. Batching is
not permission to disable `fsync`, TRIM, swap safety, or filesystem integrity.
