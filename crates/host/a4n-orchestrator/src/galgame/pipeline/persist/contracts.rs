use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSourceSpan {
    pub id: String,
    pub text_anchor_id: i64,
    pub source_snapshot_chapter_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgameScriptPersistPlan {
    pub scenes: Vec<GalgamePersistScene>,
}

pub(super) struct GalgameBranchSceneContext<'a> {
    pub(super) source_snapshot_chapter_id: i64,
    pub(super) location: Option<String>,
    pub(super) time_of_day: Option<String>,
    pub(super) source_scene_summary: Option<String>,
    pub(super) branch_lines_by_id: &'a std::collections::BTreeMap<
        &'a str,
        &'a super::super::super::GalgamePipelineBranchSceneLineStaged,
    >,
    pub(super) source_span_by_id:
        &'a std::collections::BTreeMap<&'a str, &'a GalgamePipelineSourceSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistScene {
    pub scene_index: i32,
    pub source_snapshot_chapter_id: i64,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: Value,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_index: Option<i32>,
    pub lines: Vec<GalgamePersistLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistLine {
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
    pub choices: Vec<GalgamePersistChoice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistChoice {
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_index: i32,
}
