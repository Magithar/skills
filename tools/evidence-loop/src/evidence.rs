use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub struct RawStructure {
    pub components: u64,
    pub rows: u64,
    pub terminus: String,
}

/// Mechanical structural check of a raw artifact. Recognizes two JSON
/// shapes: a bare array of rows (`components = 1`), and an object mapping
/// component name to an array of rows (`components = key count`). Anything
/// else is recorded honestly as unstructured rather than guessed at.
pub fn inspect_structure(bytes: &[u8]) -> RawStructure {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(serde_json::Value::Array(arr)) => RawStructure {
            components: 1,
            rows: arr.len() as u64,
            terminus: if arr.is_empty() { "empty".into() } else { "verified".into() },
        },
        Ok(serde_json::Value::Object(map)) => {
            let mut rows = 0u64;
            for v in map.values() {
                rows += match v {
                    serde_json::Value::Array(a) => a.len() as u64,
                    _ => 1,
                };
            }
            RawStructure {
                components: map.len() as u64,
                rows,
                terminus: if map.is_empty() { "empty".into() } else { "verified".into() },
            }
        }
        _ => RawStructure {
            components: 0,
            rows: 0,
            terminus: "unstructured".into(),
        },
    }
}

pub fn hash_file(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub fn read_bytes(path: &Path) -> anyhow::Result<Vec<u8>> {
    fs::read(path).map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))
}
