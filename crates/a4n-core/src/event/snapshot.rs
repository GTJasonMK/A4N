//! 内容寻址快照存储 trait
//!
//! 提供 SHA256 内容寻址的快照存储和增量差异计算。

use crate::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 快照条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// 内容哈希（SHA256，用于去重）
    pub content_hash: String,
    /// 实体类型
    pub entity_type: String,
    /// 实体 ID
    pub entity_id: i64,
    /// 关联的事件序号
    pub sequence: i64,
    /// 快照数据
    pub data: serde_json::Value,
    /// 创建时间
    pub created_at: chrono::NaiveDateTime,
}

/// 快照差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDelta {
    pub from_hash: String,
    pub to_hash: String,
    pub delta: serde_json::Value,
}

/// 快照存储 trait
///
/// 基于 SHA256 内容寻址，相同内容自动去重。
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    /// 存储快照（如果内容哈希已存在则跳过）
    async fn store_snapshot(
        &self,
        entity_type: &str,
        entity_id: i64,
        sequence: i64,
        data: serde_json::Value,
    ) -> Result<Snapshot, AppError>;

    /// 加载快照
    async fn load_snapshot(&self, content_hash: &str) -> Result<Option<Snapshot>, AppError>;

    /// 检查快照是否存在
    async fn exists(&self, content_hash: &str) -> Result<bool, AppError>;

    /// 计算两个快照之间的差异
    async fn delta_between(
        &self,
        from_hash: &str,
        to_hash: &str,
    ) -> Result<SnapshotDelta, AppError>;

    /// 垃圾回收：删除未引用的快照
    async fn gc(&self, referenced_hashes: &[String]) -> Result<usize, AppError>;
}
