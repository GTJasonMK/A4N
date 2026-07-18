//! 快照内容哈希。

use super::Sha256SnapshotStore;
use a4n_core::error::AppError;

impl Sha256SnapshotStore {
    pub(super) fn compute_hash(data: &serde_json::Value) -> Result<String, AppError> {
        crate::character::character_snapshot_content_hash(data).map_err(AppError::Serialization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compute_hash_is_deterministic() {
        let data = json!({
            "name": "阿青",
            "traits": ["calm", "precise"],
            "level": 3
        });

        let first = Sha256SnapshotStore::compute_hash(&data).expect("hash");
        let second = Sha256SnapshotStore::compute_hash(&data).expect("hash");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn compute_hash_distinguishes_snapshot_content() {
        let first = Sha256SnapshotStore::compute_hash(&json!({"level": 3})).expect("hash");
        let second = Sha256SnapshotStore::compute_hash(&json!({"level": 4})).expect("hash");

        assert_ne!(first, second);
    }
}
