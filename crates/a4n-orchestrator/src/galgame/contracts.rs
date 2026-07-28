use super::{
    GalgameLinePresentationDocumentV2, GalgamePipelinePromptChapter, GalgamePipelineSourceSpan,
};
use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{workflow_checkpoint, workflow_run},
    media::{media_asset, media_binding},
};
use a4n_core::shooting_script::ChapterShootingScriptV1;
use serde::{Deserialize, Serialize};

pub const PROJECT_KIND_GALGAME: &str = "galgame";
pub(super) const SCRIPT_STATUS_READY: &str = "ready";
pub const SCENE_ENTRY_MAIN: &str = "main";
pub const SCENE_ENTRY_BRANCH: &str = "branch";
pub const LINE_TYPE_NARRATION: &str = "narration";
pub const LINE_TYPE_DIALOGUE: &str = "dialogue";
pub const LINE_TYPE_MONOLOGUE: &str = "monologue";
pub const LINE_TYPE_STAGE_DIRECTION: &str = "stage_direction";
pub const LINE_TYPE_CHOICE: &str = "choice";
pub const GALGAME_TARGET_BACKGROUND: &str = "galgame_background";
pub const GALGAME_TARGET_CG: &str = "galgame_cg";
pub const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
pub const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
pub const GALGAME_ENTITY_LINE: &str = "galgame_line";
pub const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
pub const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];
pub const GALGAME_EXPRESSIONS: &[&str] = &[
    "neutral",
    "smile",
    "sad",
    "angry",
    "surprised",
    "shy",
    "serious",
];

#[derive(Debug, Clone)]
pub struct GalgamePipelineStatusReadModel {
    pub run: Option<workflow_run::Model>,
    pub checkpoints: Vec<workflow_checkpoint::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameScriptDetail {
    pub script: galgame_script::Model,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
    pub text_anchors: Vec<GalgameTextAnchorSummary>,
    pub media_bindings: Vec<GalgameMediaBindingSummary>,
    pub scenes: Vec<GalgameSceneDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameSpeakerProfile {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameTextAnchorSummary {
    pub id: i64,
    pub source_snapshot_chapter_id: Option<i64>,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameSceneDetail {
    pub scene: galgame_scene::Model,
    pub lines: Vec<galgame_line::Model>,
    pub choices: Vec<galgame_choice::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameMediaBindingSummary {
    pub binding: media_binding::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalgamePipelinePromptContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub chapters: Vec<GalgamePipelinePromptChapter>,
    pub source_spans: Vec<GalgamePipelineSourceSpan>,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
    pub shooting_scripts: Vec<GalgameChapterShootingScript>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalgameChapterShootingScript {
    pub source_chapter_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub chapter_number: i32,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameLineInput {
    pub line_id: i64,
    pub line_type: Option<String>,
    pub speaker_profile_id: Option<Option<i64>>,
    pub text: Option<String>,
    pub presentation: Option<GalgameLinePresentationDocumentV2>,
    pub text_anchor_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameSceneInput {
    pub scene_id: i64,
    pub title: Option<String>,
    pub location: Option<Option<String>>,
    pub time_of_day: Option<Option<String>>,
    pub background_note: Option<Option<String>>,
    pub next_scene_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsertGalgameLineInput {
    pub scene_id: i64,
    pub after_line_id: Option<i64>,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub presentation: GalgameLinePresentationDocumentV2,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderGalgameLinesInput {
    pub scene_id: i64,
    pub line_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoicesInput {
    pub line_id: i64,
    pub choices: Vec<UpdateGalgameChoiceOptionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoiceOptionInput {
    pub option_text: String,
    pub target_scene_id: i64,
}
