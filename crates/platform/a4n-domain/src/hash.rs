use sha2::{Digest, Sha256};

/// Stable SHA-256 fingerprint for text crossing persistence or provider boundaries.
pub fn source_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}
