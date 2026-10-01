use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn canonical_json_bytes<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize,
{
    serde_json_canonicalizer::to_vec(value)
}

pub fn canonical_json_text<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    serde_json_canonicalizer::to_string(value)
}

pub fn canonical_json_sha256<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    let canonical = canonical_json_bytes(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    Ok(format!("{:x}", hasher.finalize()))
}
