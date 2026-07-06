use super::Sha256SnapshotStore;
use a4n_core::error::AppError;
use sha2::{Digest, Sha256};

impl Sha256SnapshotStore {
    pub(super) fn compute_hash(data: &serde_json::Value) -> Result<String, AppError> {
        let json = serde_json::to_string(data).map_err(AppError::Serialization)?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}
