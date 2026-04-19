# Synapse Developer Guide

Opinionated best-practices assembled from the Rust top-50 (tokio, serde, ripgrep, clap, bevy, deno, diesel, fastembed, polars, tantivy, …). Everything here is wired into CI and runs locally with `cargo`.

## Toolchain

- `rust-toolchain.toml` pins **1.95.0**. Upgrade in lockstep across the workspace.
- `rustfmt` + `clippy` + `rust-src` come with the toolchain pin. No manual install.
- MSRV advertised in `clippy.toml` is held for one release cycle behind HEAD.

## Local Loop

```bash
cargo fmt --all                        # format
cargo clippy --all-targets --all-features -- -D warnings   # lint
cargo test --workspace                 # tests, default features
cargo test -p synapse-core --features fts-tantivy          # FTS path
cargo bench -p synapsed                # perf (opt-in)
```

## Release-profile tuning

`[profile.release]` in the workspace `Cargo.toml` sets:

| Knob | Value | Why |
|------|-------|-----|
| `lto` | `thin` | 5–10 % perf; acceptable link cost |
| `codegen-units` | `1` | max inlining; slower compile, faster runtime |
| `strip` | `symbols` | ~40 % smaller binary |
| `panic` | `abort` | no unwinding; smaller + faster |
| `opt-level` | `3` | default for release, explicit for clarity |

## `.cargo/config.toml`

- `registries.crates-io.protocol = "sparse"` — 3× faster resolution.
- `net.git-fetch-with-cli = true` — faster private-registry fetches when needed.
- Linux stanza ready for **mold** (`cargo install --git https://github.com/rui314/mold`).
- macOS stanza ready for **sccache** (`cargo install sccache`).

## Supply-chain monitoring

| Tool | Purpose | Wired via |
|------|---------|-----------|
| `cargo audit` | CVE scan against RustSec DB | `.github/workflows/rust-ci.yml` job `audit` |
| `cargo deny` | license + ban + source policy | `deny.toml` + CI job `deny` |
| `cargo outdated` | newer versions | scheduled Monday job |
| `cargo bloat` | binary-size breakdown | scheduled Monday job |
| `renovate` | auto PRs for updates | `renovate.json` |
| `rustsec/audit-check` | fails PR on new advisory | `audit` job |

## Useful local tools (not in CI)

| Command | Purpose |
|---------|---------|
| `cargo flamegraph -p synapsed` | perf profile, no code change |
| `cargo expand -p synapse-core` | see macro output |
| `cargo machete` | find unused deps |
| `cargo +nightly udeps` | same, nightly-only |
| `cargo tree --duplicates` | find dupe versions |
| `cargo depgraph` | render a DOT dep graph |
| `cargo-msrv` | find minimum supported rustc |
| `cargo release` | orchestrated tagging |
| `cargo nextest run` | 3× faster test runner |

Install with `cargo install --locked <name>`; none required for CI.

## Workspace hygiene

- `[workspace.dependencies]` holds every direct dep as a single source of truth.
- Leaf crates write `foo.workspace = true` to opt in; keeps versions lock-step.
- `Cargo.lock` is committed (workspace has binaries — required).
- `.cargo/config.toml` is committed; it carries no secrets.
- `deny.toml` bans `openssl` in favour of `rustls` — pure-Rust supply chain.

## Adding a dependency

1. `cargo add -p synapse-core <crate>` — pick the crate.
2. If used across crates, promote to `[workspace.dependencies]` and switch to
   `name.workspace = true`.
3. Run `cargo deny check` and `cargo audit` locally before pushing.
4. Renovate will open PRs for updates weekly.

## Adding a feature flag

1. Declare in `[features]` of the owning crate.
2. Gate code with `#[cfg(feature = "…")]`, fall back to a `stub` module when disabled.
3. Add a matrix entry in `.github/workflows/rust-ci.yml` → `test` job.
4. Document in this file.

## Security defaults

- `panic = "abort"` — no unwinding exposes fewer code paths.
- `rustls` over `openssl` — smaller, memory-safe TLS stack.
- Ed25519 + BLAKE3 for all signing/hashing — modern, fast, audited.
- `cargo audit` + `cargo deny` enforced in CI.
- Dependabot/Renovate for timely patching.

## Profiling cheat-sheet

```bash
# Hot-path sampling
cargo flamegraph --bin synapsed --features fts-tantivy

# Binary size by crate
cargo bloat --release -p synapsed --crates

# Duplicate dep detection
cargo tree --workspace --duplicates

# Unused deps
cargo machete
```
