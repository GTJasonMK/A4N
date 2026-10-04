use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyFactFeedbackSettingsRecord {
    pub project_id: i64,
    pub interval_chapters: i32,
    pub consolidated_sources_json: serde_json::Value,
    pub last_consolidated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMaterialFactChangeRecord {
    pub id: i64,
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub change_key: String,
    pub material_kind: String,
    pub target_id: Option<i64>,
    pub field_path: String,
    pub operation: String,
    pub before_json: serde_json::Value,
    pub after_json: serde_json::Value,
    pub source_refs_json: serde_json::Value,
    pub evidence_json: serde_json::Value,
    pub reason: String,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub invalidated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyForeshadowingRecord {
    pub id: i64,
    pub project_id: i64,

    /// 伏笔描述
    pub description: String,

    /// 埋设章节
    pub setup_chapter: i32,

    /// 揭示章节
    pub payoff_chapter: Option<i32>,

    /// 规范值由 `a4n_domain::character::ForeshadowingStatus` 定义。
    pub status: String,

    /// 关联角色
    pub related_characters: Option<serde_json::Value>,

    /// 重要程度（1~5）
    pub importance: i32,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyAttributeEventRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub timestamp: chrono::NaiveDateTime,

    /// 属性分类：explicit / implicit / social
    pub category: String,

    /// 属性键名
    pub key: String,

    /// 操作类型：add / modify / delete
    pub operation: String,

    /// 变更摘要
    pub change_description: String,

    /// 变更前值
    pub old_value: serde_json::Value,

    /// 变更后值
    pub new_value: serde_json::Value,

    /// 触发原因
    pub cause: String,

    /// 原文证据
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySnapshotReferenceRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,

    /// SHA256 哈希
    pub snapshot_hash: String,

    /// 引用计数
    pub reference_count: i32,

    /// 快照内容
    pub data: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyCharacterStateIndexRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,

    /// 角色状态摘要
    pub state_summary: String,

    /// 关键属性快照
    pub key_attributes: serde_json::Value,

    /// 情绪状态
    pub emotional_state: Option<String>,

    /// 当前位置
    pub current_location: Option<String>,

    /// 当前目标
    pub current_goal: Option<String>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyBehaviorRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    /// 记录来源：user / ai_sync / legacy_unknown。
    pub source_kind: String,

    pub behavior_description: String,

    /// 行为类型
    pub behavior_type: String,

    /// 原文摘录，作为行为依据
    pub original_text: String,

    /// 行为标签
    pub behavior_tags: serde_json::Value,

    /// 隐性属性分类结果
    pub classification_results: serde_json::Value,

    /// 相关角色
    pub related_characters: Option<serde_json::Value>,

    /// 影响分析
    pub impact_analysis: Option<String>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyDeletionMarkRecord {
    pub id: i64,
    pub profile_id: i64,

    /// 删除候选来源：user / ai_sync / legacy_unknown
    pub source_kind: String,

    /// 属性分类：explicit / implicit / social
    pub category: String,

    /// 属性键名
    pub key: String,

    /// 删除候选出现的章节号
    pub chapter_number: i32,

    /// 删除原因
    pub reason: String,

    /// 原文证据
    pub evidence: String,

    /// 连续命中计数
    pub consecutive_count: i32,

    /// 是否已执行删除
    pub is_executed: bool,

    /// 最后一次命中删除候选的章节号
    pub last_marked_chapter: i32,

    pub created_at: chrono::NaiveDateTime,

    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFamilyFactFeedbackArchive {
    pub settings: Option<ProjectFamilyFactFeedbackSettingsRecord>,
    pub changes: Vec<ProjectFamilyMaterialFactChangeRecord>,
    pub foreshadowing: Vec<ProjectFamilyForeshadowingRecord>,
    pub attribute_events: Vec<ProjectFamilyAttributeEventRecord>,
    pub snapshots: Vec<ProjectFamilySnapshotReferenceRecord>,
    pub state_indexes: Vec<ProjectFamilyCharacterStateIndexRecord>,
    pub behaviors: Vec<ProjectFamilyBehaviorRecord>,
    pub deletion_marks: Vec<ProjectFamilyDeletionMarkRecord>,
}
