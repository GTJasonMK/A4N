use a4n_domain::{galgame::GalgameLineRecord, media::MediaProvenanceRecord};
use sea_orm::FromQueryResult;

#[derive(Debug, Clone)]
pub struct GalgameMediaBindingResult {
    pub binding: GalgameMediaBindingReadRecord,
    pub asset: GalgameMediaAssetReadRecord,
}

#[derive(Debug, Clone, Default)]
pub struct GalgameMediaBindingReadModel {
    pub bindings: Vec<GalgameMediaBindingReadRecord>,
    pub assets: Vec<GalgameMediaAssetReadRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameMediaBindingReadRecord {
    pub id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameMediaAssetReadRecord {
    pub id: i64,
    pub media_kind: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GalgamePipelinePromptContextRecord {
    pub chapters: Vec<GalgamePipelinePromptChapterRecord>,
    pub speaker_profiles: Vec<GalgamePipelinePromptSpeakerProfileRecord>,
}

#[derive(Debug, Clone)]
pub struct GalgamePipelinePromptChapterRecord {
    pub chapter: GalgamePipelinePromptChapterSourceRecord,
    pub text_anchors: Vec<GalgamePipelinePromptTextAnchorRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePipelinePromptChapterSourceRecord {
    pub id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub summary: Option<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePipelinePromptTextAnchorRecord {
    pub id: i64,
    pub source_snapshot_chapter_id: Option<i64>,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePipelinePromptSpeakerProfileRecord {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct GalgamePersistValidationScopeReadModel {
    pub source_snapshot_chapters: Vec<GalgamePersistValidationSourceSnapshotChapterRecord>,
    pub speaker_profiles: Vec<GalgamePersistValidationSpeakerProfileRecord>,
    pub text_anchors: Vec<GalgamePersistValidationTextAnchorRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePersistValidationSourceSnapshotChapterRecord {
    pub id: i64,
    pub snapshot_id: i64,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePersistValidationSpeakerProfileRecord {
    pub id: i64,
    pub project_id: i64,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgamePersistValidationTextAnchorRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct GalgameScriptDetailReadModel {
    pub script: GalgameScriptDetailScriptRecord,
    pub speaker_profiles: Vec<GalgamePipelinePromptSpeakerProfileRecord>,
    pub text_anchors: Vec<GalgamePipelinePromptTextAnchorRecord>,
    pub media_bindings: GalgameMediaBindingReadModel,
    pub scenes: Vec<GalgameScriptDetailSceneRecord>,
    pub lines: Vec<GalgameLineRecord>,
    pub choices: Vec<GalgameScriptDetailChoiceRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameScriptDetailScriptRecord {
    pub id: i64,
    pub root_project_id: i64,
    pub title: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameScriptDetailSceneRecord {
    pub id: i64,
    pub source_snapshot_chapter_id: i64,
    pub adaptation_plan_id: Option<i64>,
    pub adaptation_unit_id: Option<String>,
    pub scene_index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_id: Option<i64>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameScriptDetailChoiceRecord {
    pub id: i64,
    pub line_id: i64,
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct GalgameAssetPlanningReadModel {
    pub scenes: Vec<GalgameAssetPlanningSceneRecord>,
    pub lines: Vec<GalgameLineRecord>,
    pub source_snapshot_chapters: Vec<GalgameAssetPlanningSourceChapterRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameAssetPlanningSceneRecord {
    pub id: i64,
    pub source_snapshot_chapter_id: i64,
    pub scene_index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_id: Option<i64>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameAssetPlanningSourceChapterRecord {
    pub id: i64,
    pub snapshot_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
pub struct CompletedGalgameVisualBindingRecords {
    pub binding: GalgameCompletedVisualBindingRecord,
    pub asset: GalgameCompletedVisualAssetRecord,
    pub provenance: MediaProvenanceRecord,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameCompletedVisualBindingRecord {
    pub id: i64,
    pub media_asset_id: i64,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct GalgameCompletedVisualAssetRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub media_kind: String,
    pub status: String,
    pub file_path: Option<String>,
}
