//! 快照存储实现
//!
//! 基于 protagonist_snapshot 表实现 SnapshotStore trait。

mod diff;
mod hash;
mod persistence;
mod store_impl;

use crate::SharedDatabaseConnection;
use a4n_core::error::AppError;

/// SHA256 快照存储
pub struct Sha256SnapshotStore {
    db: SharedDatabaseConnection,
}

impl Sha256SnapshotStore {
    pub fn new(db: SharedDatabaseConnection) -> Self {
        Self { db }
    }

    pub fn diff_snapshot_data(
        from: &serde_json::Value,
        to: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        Self::diff_json(from, to)
    }
}
