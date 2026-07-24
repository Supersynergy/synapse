# Bounded write-back

High-frequency hooks should not start one process and one durable SQLite
transaction per event. This optional Unix integration keeps a small,
deduplicated queue in RAM and periodically calls `synx put-batch`.

Defaults:

- flush after 120 seconds, 64 items, or 4 MiB
- one `synx` process and one SQLite transaction per flush
- SHA-256 idempotency key over the canonical item
- owner-only socket and state (`0600` files, `0700` directory)
- locked, fsynced spill only when the buffer/downstream is unavailable or full
- 30-second retry backoff after downstream failure

This integration supports macOS and Linux. `synx put-batch` itself is portable.

## Run

```sh
export SYNAPSE_DB="$HOME/.synapse/brain.db"
python3 integrations/writeback/synapse_writeback.py serve
```

From a high-frequency hook:

```sh
printf '%s' 'small searchable event' |
  python3 integrations/writeback/synapse_writeback.py enqueue \
    --title 'agent:event' \
    --meta '{"source":"agent-hook","kind":"session"}'
```

Submit several events through one local request:

```sh
printf '%s\n' \
  '{"title":"one","text":"alpha","meta":{"kind":"fact"}}' \
  '{"title":"two","text":"beta","meta":{"kind":"decision"}}' |
  python3 integrations/writeback/synapse_writeback.py enqueue-jsonl
```

Inspect or force the boundary:

```sh
python3 integrations/writeback/synapse_writeback.py status
python3 integrations/writeback/synapse_writeback.py flush
```

Applications that already have an in-process queue can skip the helper and
write JSONL directly:

```sh
printf '%s\n' \
  '{"title":"one","text":"alpha","meta":{"kind":"fact"}}' \
  '{"title":"two","text":"beta","meta":{"kind":"decision"}}' |
  synx -f "$HOME/.synapse/brain.db" put-batch
```

## Durability contract

Use direct `synx put`/`remember` for secrets, user confirmations, payments,
checkpoints, or anything that must survive immediate power loss. The normal
write-back window is intentionally RAM-only and can lose at most that pending
window on sudden host power loss.

If the buffer, `synx`, or SQLite is unavailable, pending items move to the
locked spill file. A downstream failure is spilled before the next retry, so a
hard buffer crash can recover it. Replay is idempotent with Synapse's content
hash deduplication.

The spill is an exception path, not a second always-on database. A successful
commit removes matching spill records atomically.

## Configuration

| Variable | Default |
|---|---:|
| `SYNAPSE_WRITEBACK_FLUSH_SECONDS` | `120` |
| `SYNAPSE_WRITEBACK_RETRY_SECONDS` | `30` |
| `SYNAPSE_WRITEBACK_MAX_ITEMS` | `64` |
| `SYNAPSE_WRITEBACK_MAX_BYTES` | `4194304` |
| `SYNAPSE_WRITEBACK_MAX_ITEM_BYTES` | `262144` |
| `SYNAPSE_WRITEBACK_DOWNSTREAM_TIMEOUT` | `60` |
| `SYNAPSE_WRITEBACK_SYNX` | `synx` |
| `SYNAPSE_WRITEBACK_SOCKET` | temp directory + current uid |
| `SYNAPSE_WRITEBACK_STATE_DIR` | `$XDG_STATE_HOME` or `~/.local/state` |

Test the transaction, offline recovery, and failure-before-hard-crash paths:

```sh
python3 integrations/writeback/test_writeback.py -v
```
