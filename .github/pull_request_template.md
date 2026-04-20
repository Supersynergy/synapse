<!-- thanks for sending a PR — please keep the diff focused -->

## What this changes

<!-- one or two sentences -->

## Checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace` passes
- [ ] `cargo test -p synapse-core --features full` passes (if core touched)
- [ ] `CHANGELOG.md` entry added under the current `## vNEXT` block
- [ ] Docs updated if a public API, file format, or bench number changed
- [ ] No new crate dep unless justified in the PR description
