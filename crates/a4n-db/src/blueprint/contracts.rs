use a4n_domain::project::ProjectStatus;
use chrono::NaiveDateTime;
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum BlueprintDbError {
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
}

/// 生成路径用的蓝图主体字段。
#[derive(Debug, Clone)]
pub struct NovelBlueprintUpsertInput {
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub needs_part_outlines: bool,
    pub total_chapters: i32,
    pub chapters_per_part: Option<i32>,
    pub current_version: String,
    pub now: NaiveDateTime,
}

/// 用户直接录入或编辑正式蓝图时可修改的字段。
#[derive(Debug, Clone)]
pub struct ManualBlueprintUpsertInput {
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub needs_part_outlines: bool,
    pub total_chapters: i32,
    pub chapters_per_part: Option<i32>,
    pub current_version: String,
    pub now: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct SaveManualBlueprintInput {
    pub project_id: i64,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub total_chapters: i32,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateBlueprintWorldSettingInput {
    pub project_id: i64,
    pub world_setting: Value,
    pub created_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintMutationStatusChange {
    pub from: ProjectStatus,
    pub to: ProjectStatus,
    pub changed_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct VersionedBlueprintMutationResult {
    pub blueprint: a4n_core::models::project::novel_blueprint::Model,
    pub status_change: Option<BlueprintMutationStatusChange>,
}

/// 优化路径的可选补丁字段。
#[derive(Debug, Clone)]
pub struct BlueprintRefinePatchInput {
    pub style: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub current_version: String,
    pub now: NaiveDateTime,
}

/// 蓝图版本快照写入输入。
#[derive(Debug, Clone)]
pub struct InsertBlueprintVersionInput {
    pub project_id: i64,
    pub version: String,
    pub change_description: String,
    pub blueprint_data: Value,
    pub now: NaiveDateTime,
    pub created_by: String,
}

/// 蓝图生成完成后同步项目主表字段。
#[derive(Debug, Clone)]
pub struct SyncProjectAfterBlueprintInput {
    pub project_id: i64,
    pub title: String,
    pub status: ProjectStatus,
    pub target_chapters: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub raw_blueprint: String,
    pub now: NaiveDateTime,
}
