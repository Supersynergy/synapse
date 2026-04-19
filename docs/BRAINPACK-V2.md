# `.brainpack` v2 — Distribution Format

**Status**: Stable (v0.2, 2026-04-20) · **License**: MIT · **Author**: Maxim Supersynergy

A `.brainpack` is the shippable wrapper for a Synapse store. Works as:

- **bare** — the file is a finalised `.synx` byte-for-byte (fastest to mount)
- **wrapped** — the file is a zstd stream whose decompressed body is a `.synx`
  (smallest on-wire size, one extra decode step on open)

Readers auto-detect by sniffing the first 4 bytes: `SYNX` → bare, else zstd.

## Why

- **Sell memory packs.** One signed file, drop-in for any MCP-compatible agent. Ship DSGVO-compliance packs, medical knowledge, legal precedents, DACH leads.
- **Commit to git.** `.claude/brain.brainpack` — per-project AI memory versioned with the code.
- **Offline bundles.** Full documentation set as one searchable file.
- **Portable.** Same file works on macOS/Linux/Windows, x86/ARM, no engine install required beyond the reader.

## Relationship to `.synx`

| File | Role | Mutable | Signed | Size |
|------|------|---------|--------|------|
| `brain.synx` | **live store** — written to as agents add memory | yes (CoW + journal) | optional | +/- 10–20 % over payload |
| `brain.brainpack` | **distribution** — snapshot, shipped, read-only on consumer | no | yes (recommended) | 3–6× smaller than `.synx` |
| `brain.db` | **legacy** — SQLite v1 format, import-only path | yes (WAL) | no | roughly 2× `.synx` |

## CLI (planned v0.2 surface)

```bash
# create a live store
synapse init brain.synx

# export a shippable pack (optionally signed)
synapse pack brain.synx -o release.brainpack --sign

# open a pack for reads (mmap, zero-copy once unwrapped)
synapse open release.brainpack

# one-way import from legacy v1
synapse migrate brain.db -o brain.synx
```

## Rust API

```rust
use synapse_core::BrainPack;

// ship
let bytes = BrainPack::pack("brain.synx", "release.brainpack")?;
println!("shipped {} bytes", bytes);

// consume
BrainPack::unpack("release.brainpack", "local.synx")?;
```

## Verification

Each `.brainpack` whose underlying `.synx` carries the `SIGNED` header flag
includes an Ed25519 signature over the manifest hash in the 256-byte footer.
Verify with:

```rust
use synapse_core::synx::SynxReader;

let r = SynxReader::open("received.brainpack")?;   // open also accepts wrapped
match (r.footer.signature, r.footer.pubkey) {
    (Some(sig), Some(pk)) => {
        // pubkey is Ed25519; sig is over footer.manifest_hash
        // use ed25519-dalek to verify against your trust root
    }
    _ => eprintln!("unsigned pack"),
}
```

## Compatibility Matrix

| Producer | Consumer | Supported? |
|----------|----------|------------|
| v0.2 `.synx` bare → v0.2 reader | any | ✅ |
| v0.2 zstd-wrapped → v0.2 reader | any | ✅ |
| v0.1 `.brainpack` (= zstd of SQLite `.db`) → v0.2 reader | ⚠️ legacy path via `synapse migrate` |
| any `.db` → v0.2 reader | via migrate only |
| `.synx` bare → v0.1 reader | ❌ no synx support |

## Size Expectations

Observed on real corpora (M4 Max, zstd level 19):

| Content | `.synx` bare | `.brainpack` wrapped | ratio |
|---------|--------------|----------------------|-------|
| 1 000 short docs (no embed) | ~ 1.0 MB | ~ 250 KB | 4.0× |
| 1 000 docs + 384-d embeddings | ~ 2.5 MB | ~ 1.6 MB | 1.6× |
| 10 000 short docs | ~ 1.7 MB | ~ 450 KB | 3.8× |

Embeddings are already high-entropy; the wrap mostly compresses the row metadata.

## Stability

The bare `.synx` byte layout is specified in [SYNX-FORMAT-V2.md](./SYNX-FORMAT-V2.md)
and is considered **stable from v0.2 onward**. Any breaking change bumps the
header version field; readers MUST refuse unknown versions.

Brainpack wrap itself is just "zstd of the spec-stable body" — no additional
format surface to break.
