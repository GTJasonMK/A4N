use a4n_core::{error::AppError, source_spans::SourceSpan};
use a4n_db::{
    chapter_visual_plan::ParsedChapterVisualPlan, project::ResolvedChapterSource,
    shooting_script::ParsedChapterShootingScript,
};
use a4n_domain::chapter_visual_plan::ChapterVisualPlanV1;
use a4n_domain::shooting_script::{
    ChapterShootingScriptV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
    ShootingTransitionContractV1,
};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ChapterScreenplaySkeletonV1 {
    pub(super) schema_version: String,
    pub(super) directing_statement: ShootingDirectingStatementV1,
    pub(super) units: Vec<ScreenplaySkeletonUnitV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct ScreenplayUnitAdaptationV1 {
    pub(super) schema_version: String,
    pub(super) narrative_unit_index: u32,
    pub(super) performance_facts: Vec<ScreenplayPerformanceFactV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone)]
pub(super) struct RegenerateChapterShootingScriptInput {
    pub(super) project_id: i64,
    pub(super) source_chapter_id: i64,
    pub(super) workflow_run_id: i64,
    pub(super) visual_plan_id: i64,
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
pub(super) struct PreparedChapterShootingScriptRequest {
    pub(super) source: ResolvedChapterSource,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) input_fingerprint_sha256: String,
    pub(super) existing: Option<ParsedChapterShootingScript>,
    pub(super) visual_plan: ParsedChapterVisualPlan,
    pub(super) visual_plan_projection: ChapterVisualPlanV1,
    pub(super) layout_unit_system_prompt: String,
    pub(super) unit_adaptation_system_prompt: String,
    pub(super) design_unit_system_prompt: String,
}

#[cfg(test)]
pub(crate) fn visual_plan_fixture(source: &ResolvedChapterSource) -> ParsedChapterVisualPlan {
    use a4n_domain::chapter_visual_plan::{
        CHAPTER_VISUAL_PLAN_V2, ChapterVisualPlanAuthoringMode, ChapterVisualPlanSchemaVersion,
        ChapterVisualPlanStatus, VisualCameraStateV1, VisualFrameV2, VisualKeyframeV2,
        VisualSceneV2, VisualShotV2,
    };
    use a4n_domain::shooting_script::{ShootingCoverageV1, ShootingEditingModeV1};

    let keyframe = |id: &str, at_ms: u64| VisualKeyframeV2 {
        id: id.into(),
        at_ms,
        label: id.into(),
        interpolation_to_next: a4n_domain::chapter_visual_plan::VisualInterpolationV1::Hold,
        camera: VisualCameraStateV1 {
            center_x: 0.5,
            center_y: 0.5,
            zoom: 1.0,
            rotation_degrees: 0.0,
            focus_entity_id: None,
            framing_note: None,
        },
        entity_states: Vec::new(),
        relations: Vec::new(),
    };
    let shot = |id: &str| VisualShotV2 {
        id: id.into(),
        title: id.into(),
        duration_ms: 1_000,
        narrative_purpose: "推进".into(),
        coverage: ShootingCoverageV1::MustShow,
        shot_size: None,
        camera_angle: None,
        camera_height: None,
        lens_intent: None,
        composition: None,
        keyframes: vec![
            keyframe(&format!("{id}-start"), 0),
            keyframe(&format!("{id}-end"), 1_000),
        ],
        entity_events: Vec::new(),
        camera_events: Vec::new(),
        sound_events: Vec::new(),
        technique_events: Vec::new(),
        continuity_notes: Vec::new(),
        transition_to_next: None,
    };
    let plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV2 {
        schema_version: ChapterVisualPlanSchemaVersion::V2,
        frame: VisualFrameV2 {
            width: 1920,
            height: 1080,
            background: None,
        },
        entities: Vec::new(),
        scenes: vec![VisualSceneV2 {
            id: "scene-1".into(),
            title: "场景".into(),
            narrative_purpose: "推进".into(),
            editing_mode: ShootingEditingModeV1::Continuity,
            intercut_group_id: None,
            location: None,
            time_of_day: None,
            participant_entity_ids: Vec::new(),
            focal_entity_id: None,
            shots: vec![shot("shot-1"), shot("shot-2")],
        }],
    };
    let plan_sha256 =
        a4n_db::generation::canonical_json_sha256(&plan).expect("visual plan fixture sha256");
    let now = chrono::Utc::now().naive_utc();
    ParsedChapterVisualPlan {
        record: a4n_core::models::chapter::chapter_visual_plan::Model {
            id: 1,
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.source_snapshot_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            source_revision_key: source.source_revision_key.clone(),
            source_content_sha256: source.content_sha256.clone(),
            contract_version: CHAPTER_VISUAL_PLAN_V2.into(),
            plan_version: 1,
            revision: 1,
            status: ChapterVisualPlanStatus::Confirmed.as_str().into(),
            authoring_mode: ChapterVisualPlanAuthoringMode::Manual.as_str().into(),
            plan_sha256,
            plan_json: serde_json::to_value(&plan).expect("visual plan fixture json"),
            author_user_id: 1,
            confirmed_at: Some(now),
            created_at: now,
            updated_at: now,
        },
        status: ChapterVisualPlanStatus::Confirmed,
        authoring_mode: ChapterVisualPlanAuthoringMode::Manual,
        plan,
    }
}

#[derive(Debug, Clone)]
pub struct EditedChapterShootingScript {
    pub shooting_script: ParsedChapterShootingScript,
    pub invalidated_project_ids: Vec<i64>,
}
