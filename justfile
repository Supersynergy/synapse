# synapse-memory — agent memory (depends on synapse-db via vendor/ submodule)
set shell := ["bash", "-uc"]

default: check

# init the vendored synapse-db foundation, then verify toolchain
setup:
    git submodule update --init --recursive
    rustup show

# fmt + clippy + type check (fast gate)
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets
    cargo check --workspace

# run the test suite
test:
    cargo nextest run --workspace --no-fail-fast

ci: check test
    cargo deny check || true

fmt:
    cargo fmt --all

build:
    cargo build --workspace --release
