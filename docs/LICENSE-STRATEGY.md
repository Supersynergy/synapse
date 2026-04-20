# License strategy for Synapse v1.0

**Author**: Maxim Supersynergy · **Date**: 2026-04-20

## TL;DR recommendation

| Artefact | License | Why |
|----------|---------|-----|
| **Rust reference implementation** (`crates/*`, `sdk/*`) | **MIT** | maximum adoption; no patent grant needed (you aren't filing any) |
| **`.synx` format specification** (`docs/SYNX-FORMAT-V2.md`, `docs/BRAINPACK-V2.md`) | **CC0 / public domain** | anyone can write a reader in any language, no license friction |
| **Conformance test vectors** (future `spec/conformance/`) | **CC0** | same reason — standards must be unowned |
| **Paid `.brainpack` subscriptions** (DSGVO pack, medical pack, DACH leads pack) | **proprietary data** | the *content* is the product, not the software |

This is the **Linux / Git / Parquet** pattern. The tool is free; the value you sell sits *on top of* the tool.

## Why not X

| License | Pros | Cons | Verdict for Synapse |
|---------|------|------|---------------------|
| **MIT** | simplest, most adopted | no patent grant | ✅ chosen |
| **Apache-2.0** | explicit patent grant | 4 more paragraphs in every file header | consider only if you ever patent something |
| **MPL-2.0** | file-level copyleft | less adoption, rarely seen in Rust | no |
| **BSL (Business Source License)** | paid commercial, converts to OSS after 4 y | hostile to adoption — SurrealDB, Sentry, Cockroach all took a reputation hit | **no** — you said no lock-in |
| **AGPL** | forces network-use to open source | kills enterprise adoption | no |
| **SSPL** | Mongo-style, blocks cloud providers | OSI says it is not open source | no |
| **Commons Clause** (atop MIT) | bans selling "the software" | confusing, not OSI | no |
| **Elastic License 2.0** | vendor-friendly | same SourceAvail taint as BSL | no |
| **Custom proprietary** | total control | kills community, kills the standard play | no |
| **Dual MIT + commercial** (Sidekiq / GitLab EE) | free core + paid premium | legitimate, but only pays off once you have a non-trivial premium feature set | later — consider at v2.0 when paid tier exists |

Your north star from `feedback_opensource_preference.md`: *"Default OSS/self-host, avoid Supabase/Firebase/Clerk/Vercel-only. Always Docker-deployable."* The MIT + CC0 split honours that for your own project.

## How to monetise without closing the license

1. **`.brainpack` subscription packs** — the `.synx` format is public; the *content* (DSGVO-up-to-date rules, medical research snapshots, DACH lead databases) is proprietary. Like Bloomberg terminal: open format, paid data.
2. **Signed trust-root service** — sell enterprise customers a pinned Ed25519 pubkey they trust, ship signed packs weekly. Revenue for the signature chain, not the bits.
3. **Synapse Cloud** — hosted multi-peer sync (the CRDT part) for teams that don't want to wire it themselves. Keep the single-node use case free forever.
4. **Enterprise support contracts** — bug SLAs, custom extension modules (e.g. corporate SSO for `.brainpack` decryption keys).
5. **Consulting** — help companies move from `vector-DB-cluster + Redis + Postgres + Python` to Synapse. High-ticket, low-volume, easy conversion.

All five revenue paths coexist with MIT+CC0. None of them requires relicensing.

## How to use it right now

```bash
# 1. install (standalone binary, single file)
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo install --path crates/synapse-cli
cargo install --path crates/synapsed
cargo install --path crates/synapse-mcp

# 2. start the daemon (persistent on disk, 0.8ms/req once warm)
synapsed -f ~/.synapse/brain.db &

# 3. put a memory
synapse put "rust ships here tonight" --title "decision" --meta '{"scope":"project/supersynergy"}'

# 4. hybrid search
synapse search "where does rust ship" --mode hybrid --embed

# 5. export the whole brain as one file
synapse snap ~/.synapse/brain.brainpack
# commit or ship this file anywhere

# 6. plug into Claude Code (MCP)
cat >> ~/.claude/settings.json <<EOF
{
  "mcpServers": {
    "synapse": {
      "command": "/opt/homebrew/bin/synapse-mcp",
      "args": ["--sock", "/tmp/synapse.sock"]
    }
  }
}
EOF
# restart Claude → `put`, `search`, `stats` appear as tools
```

## Spec vs implementation — two different licenses

The `.synx` **format specification** in `docs/SYNX-FORMAT-V2.md` is released as **CC0**. That means:

- any language can write a reader; nobody needs to ask
- no warranty, no patents asserted
- the format outlives this repo

The **reference Rust implementation** is MIT. That means:

- use in commercial products, no problem
- attribution required (keep the `LICENSE` file)
- no patent grant (we aren't filing any)

This is the Parquet, the Arrow, the HLS pattern — free spec, free reference, paid content.

## Practical next steps

1. Leave the current `LICENSE` (MIT) as-is.
2. Add a line to `docs/SYNX-FORMAT-V2.md` declaring the spec as CC0 (already present).
3. When the first `.brainpack` product ships, price it as a subscription with rotating Ed25519 keys — the software stays free.
4. When enterprise sales start, offer Apache-2.0-relicensed builds if a customer demands the patent clause; it is backward-compatible with MIT so this is always available.

The license is not your moat. The combination of Rust speed + file format + 20-tool feature set in one MCP-native binary is the moat.
