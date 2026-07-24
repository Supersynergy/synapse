# synapse-memory — local-first agent memory
set shell := ["bash", "-uc"]

default: check

# verify the pinned toolchain; all public source is in this checkout
setup:
    rustup show

# fmt + clippy + type check + layering guard (fast gate)
check:
    python3 scripts/check-layering.py
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace

# enforce ADR 0001: product crates must not depend on excluded experimental crates
check-layers:
    python3 scripts/check-layering.py

# run the test suite
test:
    cargo nextest run --workspace --no-fail-fast

ci: check test
    cargo deny check || true

fmt:
    cargo fmt --all

build:
    cargo build --workspace --release
