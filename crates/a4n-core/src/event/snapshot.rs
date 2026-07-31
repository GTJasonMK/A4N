//! 内容寻址快照数据合同
//!
//! 提供快照与增量差异的跨层数据结构。

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
