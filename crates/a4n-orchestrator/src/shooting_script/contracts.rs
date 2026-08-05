use a4n_core::{
    error::AppError,
    shooting_script::{
        ChapterShootingScriptV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
        ShootingTransitionContractV1,
    },
    source_spans::SourceSpan,
};
use a4n_db::{project::ResolvedChapterSource, shooting_script::ParsedChapterShootingScript};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(super) const CHAPTER_SCREENPLAY_SKELETON_PROMPT: &str = "chapter_screenplay_skeleton";
pub(super) const CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT: &str = "chapter_screenplay_layout_unit";
pub(super) const CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT: &str =
    "chapter_screenplay_unit_adaptation";
pub(super) const CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT: &str = "chapter_shooting_design_unit";
pub(super) const CHAPTER_SHOOTING_SCRIPT_WORKFLOW: &str = "chapter_shooting_script";
pub const CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL: u32 = 100;

pub type ShootingScriptProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

/// 活体信号出口：与字段级里程碑取自同一个 token 流。
pub type ShootingScriptStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

pub(super) struct ShootingScriptStageResult<T> {
    pub(super) output: T,
    pub(super) restored: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(super) struct ChapterScreenplayPlan {
    pub(super) directing_statement: ShootingDirectingStatementV1,
    pub(super) units: Vec<ScreenplayPlanUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ChapterScreenplayLayoutV1 {
    pub(super) schema_version: String,
    pub(super) directing_statement: ShootingDirectingStatementV1,
    pub(super) units: Vec<ScreenplayLayoutUnitV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ChapterScreenplaySkeletonV1 {
    pub(super) schema_version: String,
    pub(super) directing_statement: ShootingDirectingStatementV1,
    pub(super) units: Vec<ScreenplaySkeletonUnitV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplaySkeletonUnitV1 {
    pub(super) scene_identity: String,
    pub(super) sequence_index: u32,
    pub(super) narrative_unit_index: u32,
    pub(super) timeline_position: u32,
    pub(super) intercut_group_id: Option<String>,
    pub(super) editing_mode: ShootingEditingModeV1,
    pub(super) source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayLayoutUnitDetailV1 {
    pub(super) schema_version: String,
    pub(super) narrative_unit_index: u32,
    pub(super) transition_contracts: Vec<ShootingTransitionContractV1>,
    pub(super) title: String,
    pub(super) slugline: String,
    pub(super) participants: Vec<String>,
    pub(super) dramatic_question: String,
    pub(super) scene_objective: String,
    pub(super) scene_obstacle: String,
    pub(super) entry_state: String,
    pub(super) exit_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayLayoutUnitV1 {
    pub(super) scene_identity: String,
    pub(super) sequence_index: u32,
    pub(super) narrative_unit_index: u32,
    pub(super) timeline_position: u32,
    pub(super) intercut_group_id: Option<String>,
    pub(super) editing_mode: ShootingEditingModeV1,
    pub(super) transition_contracts: Vec<ShootingTransitionContractV1>,
    pub(super) title: String,
    pub(super) slugline: String,
    pub(super) source_span_ids: Vec<String>,
    pub(super) participants: Vec<String>,
    pub(super) dramatic_question: String,
    pub(super) scene_objective: String,
    pub(super) scene_obstacle: String,
    pub(super) entry_state: String,
    pub(super) exit_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayUnitAdaptationV1 {
    pub(super) schema_version: String,
    pub(super) narrative_unit_index: u32,
    pub(super) performance_facts: Vec<ScreenplayPerformanceFactV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayPlanUnit {
    pub(super) scene_identity: String,
    pub(super) sequence_index: u32,
    pub(super) narrative_unit_index: u32,
    pub(super) timeline_position: u32,
    pub(super) intercut_group_id: Option<String>,
    pub(super) editing_mode: ShootingEditingModeV1,
    pub(super) transition_contracts: Vec<ShootingTransitionContractV1>,
    pub(super) title: String,
    pub(super) slugline: String,
    pub(super) source_span_ids: Vec<String>,
    pub(super) participants: Vec<String>,
    pub(super) dramatic_question: String,
    pub(super) scene_objective: String,
    pub(super) scene_obstacle: String,
    pub(super) entry_state: String,
    pub(super) exit_state: String,
    pub(super) performance_facts: Vec<ScreenplayPerformanceFactV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayPerformanceFactV1 {
    pub(super) fact_index: u32,
    pub(super) source_span_ids: Vec<String>,
    pub(super) actor: Option<String>,
    pub(super) objective: String,
    pub(super) obstacle: String,
    pub(super) visible_action: String,
    pub(super) reaction: Option<String>,
    pub(super) dialogue_intent: Option<String>,
    pub(super) entry_state: String,
    pub(super) exit_state: String,
    pub(super) required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterShootingScriptSourceInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegenerateChapterShootingScriptInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedChapterShootingScript {
    pub source: ResolvedChapterSource,
    pub shooting_script: ParsedChapterShootingScript,
}

#[derive(Debug, Clone)]
pub struct ChapterShootingScriptUnitProgress {
    pub workflow_run_id: i64,
    pub script: ChapterShootingScriptV1,
    pub completed_unit_count: usize,
    pub total_unit_count: usize,
    pub resolved: Option<ResolvedChapterShootingScript>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedChapterShootingScriptRequest {
    pub(crate) source: ResolvedChapterSource,
    pub(crate) source_spans: Vec<SourceSpan>,
    pub(crate) input_fingerprint_sha256: String,
    pub(crate) existing: Option<ParsedChapterShootingScript>,
    pub(crate) skeleton_system_prompt: String,
    pub(crate) layout_unit_system_prompt: String,
    pub(crate) unit_adaptation_system_prompt: String,
    pub(crate) design_unit_system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditChapterShootingScriptInput {
    pub record_id: i64,
    pub expected_revision: i32,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone)]
pub struct EditedChapterShootingScript {
    pub shooting_script: ParsedChapterShootingScript,
    pub invalidated_project_ids: Vec<i64>,
}
