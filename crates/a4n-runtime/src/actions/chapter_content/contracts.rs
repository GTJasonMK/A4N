use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ImportChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChapterContentInput {
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialSyncStatus {
    Completed,
    NotAnalyzed,
    NeedsAnalysis,
    Pending,
    Running,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChapterMaterialSyncState {
    pub content_index: ChapterMaterialSyncStatus,
    pub outline_and_foreshadowing: ChapterMaterialSyncStatus,
    pub objects: ChapterMaterialSyncStatus,
    pub characters: ChapterMaterialSyncStatus,
    pub scenes: ChapterMaterialSyncStatus,
}

impl ChapterMaterialSyncState {
    pub(super) const fn all(status: ChapterMaterialSyncStatus) -> Self {
        Self {
            content_index: status,
            outline_and_foreshadowing: status,
            objects: status,
            characters: status,
            scenes: status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialUpdateKind {
    Outline,
    Foreshadowing,
    Object,
    Character,
    Scene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialUpdateAction {
    Created,
    Updated,
    Existing,
    Preserved,
    Synced,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChapterMaterialUpdate {
    pub material_kind: ChapterMaterialUpdateKind,
    pub action: ChapterMaterialUpdateAction,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<serde_json::Value>,
    pub analysis_data: Option<serde_json::Value>,
    pub analysis_sync_status: Option<String>,
    pub analysis_sync_can_resume: bool,
    pub analysis_sync_has_saved_analysis: bool,
    pub analysis_sync_resume_block_reason: Option<String>,
    pub analysis_sync_error: Option<String>,
    pub derived_sync_status: Option<String>,
    pub derived_sync_workflow_run_id: Option<i64>,
    pub derived_sync_can_resume: bool,
    pub derived_sync_error: Option<String>,
    pub material_sync: ChapterMaterialSyncState,
    pub material_updates: Vec<ChapterMaterialUpdate>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
