# Synapse Re-release Plan — 2026-04-20

**Local repo state**: clean, tagged v0.3.1 rollup at `HEAD`. All v0.2.x tags retained in history.

**Backup**: `/tmp/synapse-final-*/` contains `repo.bundle` (git-restorable), `worktree.tar.gz`, `commits.txt`, `tags.txt`. Restore with `git clone repo.bundle synapse-restored`.

## If you delete the GitHub repo and re-push from zero

1. Delete the remote repo in GitHub UI.
2. Re-create an empty `Supersynergy/synapse` (same name or new name — your call).
3. Back in this local checkout:
   ```bash
   cd /tmp/synapse
   git remote set-url origin https://github.com/<owner>/<repo>.git
   git push -u origin main
   git push origin --tags
   ```
4. The entire v0.2.x + v0.3.x history lands intact. Every tag becomes a GitHub
   release automatically (GitHub converts annotated tags).

## If you want a clean v1.0.0 squash

```bash
git checkout --orphan v1-clean
git add -A
git commit -m "chore: clean v1.0.0 snapshot of Synapse

Synapse v1.0.0 — single-file agent memory.
- .synx container format (spec CC0, ref impl MIT)
- .brainpack distribution wrapper, optional Ed25519 signing
- Tantivy FTS + HNSW+PQ vectors + CRDT sync (Automerge)
- Python conformance reader
- 20-usecase CatBoost-tuned bench
- Supply-chain policy via deny.toml + renovate

Built in Rust. Shipped in Germany. Open forever.
"
git branch -D main
git branch -m main
git tag v1.0.0
git push --force-with-lease origin main
git push origin v1.0.0
```

The squash loses the v0.2.x commits but keeps every artefact (code, docs,
benches, history narratives in `CHANGELOG.md`).

## Files that matter for re-release narrative

| Path | Role |
|------|------|
| `README.md` | Entry point, badges, usecase list, quickstart |
| `CHANGELOG.md` | Full v0.2.0 → v0.3.1 story |
| `docs/SYNX-FORMAT-V2.md` | Binary layout spec |
| `docs/BRAINPACK-V2.md` | Distribution format |
| `docs/SYNX-IMPLEMENTATION.md` | 4-phase plan (all phases now 🟢) |
| `docs/RFC-CALL.md` | Invite other projects to review |
| `docs/STRATEGY.md` | How we top each competitor |
| `docs/USECASES.md` | 20 real-world ways to deploy |
| `docs/DEV.md` | Supply-chain + best-practices guide |
| `bench/RESULTS-V2.md` | v0.1 vs v0.2 numbers |
| `bench/RESULTS-V2-FULL.md` | 20-usecase × CatBoost |
| `bench/RESULTS-TOP20.md` | Synapse vs 15 other formats |
| `sdk/python/synapse_reader.py` | Cross-language conformance reference |

## Checklist before `git push -u origin main`

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] `cargo test --workspace` (default: 20, +fts-tantivy: 21, +full: 27)
- [x] `deny.toml` passes (supply-chain policy)
- [x] `rust-toolchain.toml` pins 1.95.0
- [x] `Cargo.toml` version = `0.3.1`
- [x] `CHANGELOG.md` entry for v0.3.1 — full roll-up
- [x] all tests touched by new modules pass
- [ ] recreate GitHub repo (user action)
- [ ] configure GitHub Actions secrets if the CI workflow needs any (none currently)
- [ ] enable Dependabot / Renovate in the new repo settings
