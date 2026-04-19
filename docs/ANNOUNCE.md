# Launch Thread — Draft

The hook, the body, and the copy for the launch post. Use verbatim or remix.

## One-liner (Twitter / LinkedIn bio)

**Synapse — one file, your AI's entire memory. 45,000× faster than MV2.**

## Long-form tweet / HN title

> Your AI agent doesn't need Qdrant + Pinecone + Redis + Postgres + a Python venv to remember things. It needs one file.
>
> I rebuilt memvid's `.mv2` format from scratch in Rust. Same single-file portability. Daemon-mode. **9,000× faster insert. 45,000× faster search.** MIT-licensed. One binary.
>
> github.com/Supersynergy/synapse

## Hacker News post

**Title:** Show HN: Synapse – single-file memory for AI agents, 45,000× faster than MV2

**Body:**
> Hi HN. Synapse is a drop-in replacement for memvid's `.mv2` format.
>
> Benchmarks on M4 Max, 1000 docs, release build, daemon mode (reproducible via `./bench/run_all.sh`):
>
> - Insert 1k docs (no embed): 147 s → **16 ms** (9,074×)
> - Lex search: 12.4 s/q → **0.275 ms/q** (45,091×)
> - Hybrid RRF (lex + vec): new capability, 1.77 ms/q
> - Re-embed cached text: full compute → **1.4 ms / 500 docs** (1,273× via BLAKE3 dedup)
> - `.brainpack` snapshot: 5.8× smaller than `.mv2` at same content
>
> The trick isn't magic. MV2 spawns a full CLI per call and rebuilds a Tantivy index. Synapse runs a Rust daemon (tokio, unix socket, msgpack-rpc) on top of SQLite + FTS5 + sqlite-vec + a redb BLAKE3 embedding cache. The single-file portability is preserved via a zstd snapshot format (`.brainpack`) with a BLAKE3 checksum.
>
> Why I built it: I wanted `git commit .claude/brain.brainpack` to Just Work for AI agent memory across projects and teammates. MV2 had the right shape. It didn't have the speed.
>
> MIT-licensed. Single-binary. No Python. No cloud. No vendor. SECURITY.md is written.
>
> Would love feedback on: the hybrid RRF scoring tuning, the `.brainpack` format (worth versioning the magic bytes further?), and whether an optional HTTP bridge is worth the surface-area increase.

## Product Hunt tagline

> The RAG stack is dead. One file. One binary. Rust-fast.

## LinkedIn post (B2B angle)

> Most companies running AI pilots are about to hit the wall where "my agent doesn't remember what happened yesterday" meets "we now have Qdrant + Redis + Postgres + three Docker composes for one chat feature."
>
> I published **Synapse** today: a single-file AI memory layer. One binary. 9,000× faster insert than the next single-file competitor, 45,000× faster search. MCP-native, so it drops straight into Claude Code and any agent that speaks MCP.
>
> It's MIT-licensed because the world doesn't need another vector DB vendor. It needs one less.
>
> Link in comments.

## The "Cloudflare move" angle

Cloudflare positioned their CMS project as the spiritual successor to WordPress by dropping the boring boilerplate (PHP, MySQL, cPanel) and keeping the part that mattered (the authoring model). The analog for us:

> **Synapse is the spiritual successor to the vector-DB stack.**
>
> Keep: hybrid search, metadata filtering, fast kNN, one-file portability.
> Drop: the server, the Docker compose, the Python runtime, the vendor lock-in.

Not subtle. Intended.

## Distribution plan

1. **Hour 0** — commit + tag `v0.1.0`; release notes = this doc.
2. **Hour 1** — HN Show HN with benchmarks above.
3. **Hour 1** — Twitter / LinkedIn with one-liner + GIF of bench running.
4. **Hour 2** — Post to r/LocalLLaMA, r/rust, r/programming. Angle per subreddit.
5. **Day 1** — DM 10 people in the MCP / Claude Code / memvid circles. Short plain-text: "built this, thought you'd want to see it, would love feedback."
6. **Day 2** — if a thread gains, publish a follow-up showing the Claude Code plugin integration end-to-end.
7. **Week 1** — ship the first 20 `.brainpack` data packs (DSGVO, BFSG, MDN, Rust std, Stripe docs, …) as a free marketplace. This is the moat.

## Metrics to hit

- 1k GitHub stars in Week 1 (MV2 took ~30 days).
- 100 HN upvotes on the Show HN (enough for front page at low traffic).
- One meaningful PR from outside (signals external trust).
- One partner integration (memvid? DuckDB? Claude Code marketplace?).

## What not to do

- Don't trash-talk memvid. They proved the idea. Credit them. Link their repo. Ecosystem > zero-sum.
- Don't overclaim on scale. Synapse is not going to replace Qdrant at 1B vectors. Say so.
- Don't make it political. Ship the tech.
