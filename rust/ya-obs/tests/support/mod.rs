//! Shared helpers for loading test-vector fixtures.

use std::path::PathBuf;

/// Path to the repo-root `test-vectors/` directory, resolved relative to the
/// `ya-obs` crate at `rust/ya-obs/`.
pub fn vectors_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // rust/
        .and_then(|p| p.parent()) // repo root
        .expect("CARGO_MANIFEST_DIR must have at least two ancestors")
        .join("test-vectors")
}

/// Read and parse a JSON test vector at `test-vectors/<category>/<name>.json`.
pub fn load_vector(category: &str, name: &str) -> serde_json::Value {
    let path = vectors_dir().join(category).join(format!("{name}.json"));
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}
