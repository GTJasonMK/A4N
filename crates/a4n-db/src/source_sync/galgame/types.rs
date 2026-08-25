use a4n_core::models::{
    galgame::{galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::FromQueryResult;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameSceneConflictCandidateRecord {
    pub scene: SourceSyncGalgameSceneConflictSceneRecord,
    pub old_source_snapshot_chapter_id: i64,
    pub line_count: u64,
    pub anchored_line_count: u64,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct SourceSyncGalgameSceneConflictSceneRecord {
    pub id: i64,
    pub scene_index: i32,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameAnchorConflictCandidateRecord {
    pub line: SourceSyncGalgameAnchorConflictLineRecord,
    pub scene_title: Option<String>,
    pub old_anchor: text_anchor::Model,
    pub candidates: Vec<text_anchor::Model>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct SourceSyncGalgameAnchorConflictLineRecord {
    pub id: i64,
    pub scene_id: i64,
    pub line_index: i32,
    pub text_anchor_id: Option<i64>,
    pub line_type: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameAnchorRecord {
    pub id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameSceneRegenerationSourceRecord {
    pub scene: galgame_scene::Model,
    pub script: galgame_script::Model,
    pub old_snapshot_chapter: source_snapshot_chapter::Model,
    pub new_snapshot_chapter: source_snapshot_chapter::Model,
    pub anchors: Vec<SourceSyncGalgameAnchorRecord>,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameSceneLineWriteRecordInput {
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameSceneWriteRecordInput {
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: Value,
    pub background_note: Option<String>,
    pub lines: Vec<SourceSyncGalgameSceneLineWriteRecordInput>,
}

#[derive(Debug, Clone)]
pub struct SourceSyncGalgameSceneRegenerationApplyRecord {
    pub galgame_scene_id: i64,
    pub old_source_snapshot_chapter_id: i64,
    pub new_source_snapshot_chapter_id: i64,
    pub old_line_count: usize,
    pub new_line_count: usize,
    pub deleted_generation_spec_count: usize,
    pub deleted_portrait_generation_spec_count: usize,
    pub resolved_related_conflict_count: u64,
}
