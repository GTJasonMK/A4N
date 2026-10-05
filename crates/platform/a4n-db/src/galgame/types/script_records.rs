use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistGalgameScriptRecordInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub branch_plan: Option<a4n_domain::media_authoring::AuthoringDocumentRef>,
    pub title: String,
    pub style_config_json: Value,
    pub status: String,
    pub scenes: Vec<PersistGalgameSceneRecordInput>,
    pub expected_story_graph_revision: i64,
    pub story_graph: a4n_domain::galgame::StoryGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistGalgameSceneRecordInput {
    pub scene_index: i32,
    pub source_snapshot_chapter_id: i64,
    pub adaptation_plan_id: Option<i64>,
    pub adaptation_unit_id: Option<String>,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: Value,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_index: Option<i32>,
    pub lines: Vec<PersistGalgameLineRecordInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistGalgameLineRecordInput {
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
    pub choices: Vec<PersistGalgameChoiceRecordInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistGalgameChoiceRecordInput {
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceGalgameChoiceRecordInput {
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertGalgameLineRecordInput {
    pub scene_id: i64,
    pub after_line_id: Option<i64>,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGalgameLineRecordInput {
    pub line_id: i64,
    pub line_type: Option<String>,
    pub speaker_profile_id: Option<Option<i64>>,
    pub text: Option<String>,
    pub expression: Option<Option<String>>,
    pub pose: Option<Option<String>>,
    pub position: Option<Option<String>>,
    pub presentation_json: Option<Value>,
    pub text_anchor_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGalgameSceneRecordInput {
    pub scene_id: i64,
    pub title: Option<String>,
    pub location: Option<Option<String>>,
    pub time_of_day: Option<Option<String>>,
    pub background_note: Option<Option<String>>,
    pub next_scene_id: Option<Option<i64>>,
}
