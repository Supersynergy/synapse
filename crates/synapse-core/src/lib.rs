//! synapse-core — single-file memory for AI agents.
//!
//! Synapse v0.2 ships **two storage engines**:
//!
//! - **`.synx` v2** (default for new stores) — Rust-native container:
//!   Arrow row-batches + Tantivy FTS (soon) + HNSW+PQ (soon) + BLAKE3 content
//!   chunks + CoW journal + optional Ed25519 footer. Spec: `docs/SYNX-FORMAT-V2.md`.
//! - **`.db` v1** (legacy compat) — SQLite + FTS5 + sqlite-vec. Still the engine
//!   behind the daemon until the v2 read-path is fully wired; serves as the
//!   one-way migration source via `synx::migrate`.
//!
//! `.brainpack` is the distribution wrapper for shipping a `.synx` store
//! (optionally zstd-wrapped, optionally signed). See `brainpack::BrainPack`.

pub mod db;
pub mod error;
pub mod snap;
pub mod types;

#[cfg(feature = "embed")]
pub mod embed;

// v2 (.synx) format — always built, no feature flag.
pub mod brainpack;
pub mod sync;
pub mod synx;

pub use brainpack::BrainPack;
pub use db::Store;
pub use error::{Error, Result};
pub use types::{Doc, Hit, PutRequest, SearchMode};
