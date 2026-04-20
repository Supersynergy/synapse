# Running Synapse as Claude Code's persistent memory

**Scope**: real agentic tests that turn Synapse into the memory layer Claude Code has been missing. Every snippet below is a `Bash`-runnable step; the numbers cited are from the v1.0 bench (`bench/RESULTS-V1.md`).

## Why Claude Code needs external memory

- 200 K-token context window per session.
- Zero automatic recall between sessions.
- Every workaround invents a new pipeline (Qdrant + Python embedder + Redis + Postgres).
- The cost is real: rebuilding project context eats ~2 K tokens every time a session starts.

Synapse replaces that pipeline with **one file on disk + one MCP bridge**.

## One-time install

```bash
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo install --locked --path crates/synapse-cli
cargo install --locked --path crates/synapsed
cargo install --locked --path crates/synapse-mcp
```

Three binaries land in `~/.cargo/bin/`:

- `synapse` — one-shot CLI (put / search / snap / stats)
- `synapsed` — persistent tokio daemon (5.7 µs RTT)
- `synapse-mcp` — stdio JSON-RPC bridge for Claude Code

## Wire it into Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "synapse": {
      "command": "synapse-mcp",
      "args": ["--sock", "/tmp/synapse.sock"]
    }
  }
}
```

Start the daemon once per machine (or add to `launchd` / `systemd`):

```bash
synapsed -f ~/.synapse/brain.db -s /tmp/synapse.sock &
```

Restart Claude Code. Three tools now appear automatically:

- `synapse__put` — write a memory (text + optional title + meta)
- `synapse__search` — lex, vec, or hybrid
- `synapse__stats` — chunk / doc counts

## Five real agentic workflows

### 1. Per-project persistent context

At the start of every session, Claude Code pulls the top memories for the project:

```
synapse__search query="project:supersynergy" limit=10 mode=Hybrid
```

Before ending a session, it writes any new decisions:

```
synapse__put text="trailbase chosen over pocketbase, reason: MIT+single-binary+SQLite"
            title="auth/backend decision"
            meta='{"scope":"project/supersynergy","date":"2026-04-20"}'
```

Next session starts already briefed — no copy-paste.

### 2. Code-change memory

After every commit, a git hook writes the commit message + touched files into
Synapse. Weeks later: "why did we rename `foo`?" resolves to the exact commit
with scope filter `project/<repo>` and the hash of the touched file as a
BLAKE3 anchor.

### 3. Scraped-knowledge bundle

```
# nightly cron, outside Claude
super-research "rust single-file db formats" --count 50 | \
    synapse put-batch --title-prefix "research:"
```

Claude Code on the next run has the digest local, searchable, offline, zero
API cost. Synapse's hybrid search fuses BM25 + vec with RRF in 1.77 ms / q.

### 4. Per-user CRM brain for agents

Each prospect gets a scope-scoped `.brainpack` (see mem0 parity in
`docs/COMPARISON-V1.md`). The sales agent loads only that scope:

```
synapse__search query="last-interaction" meta_scope="user/acme-corp" limit=5
```

Ship the `.brainpack` to teammates so they pick up the thread.

### 5. Compliance snapshot distribution

Sign and ship a DSGVO rule pack every 90 days:

```
synapse snap /tmp/dsgvo-2026-Q2.brainpack --sign --key ~/.synapse/release.key
# teammates verify against the pinned pubkey
```

## Measured agent-relevant latencies (from the v1.0 bench)

| op | Synapse v1.0 | Claude-session-level impact |
|----|-------------:|------------------------------|
| session boot — read top-10 memories | < 1 ms | imperceptible |
| put a new memory | 67 µs (bulk ~16 k docs/s) | imperceptible |
| hybrid search of all-project memory | 1.8 ms | below 1 frame |
| export whole brain as `.brainpack` | 7 ms / 350 KB | imperceptible |
| verify a signed `.brainpack` | 25 µs | imperceptible |

## Reference runner

`bench/claude_code_memory_demo.sh` drives the daemon exactly as the MCP
bridge would: three simulated sessions, five puts, three searches, one
snapshot. Use it as the template for your own CI smoke tests.

## Why this matters

Claude Code with Synapse wired in becomes the first mainstream AI coding
agent where the *memory* is:

1. **Persistent across sessions**
2. **Portable** — `git commit brain.brainpack`
3. **Searchable** — BM25 + vector + hybrid in one call
4. **Signed** — teammates verify before load
5. **Offline-first** — no vendor lock, no API costs
6. **µs-scale** — indistinguishable from in-process state

That is the shape of "an agent that remembers". Synapse v1.0 ships it today.
