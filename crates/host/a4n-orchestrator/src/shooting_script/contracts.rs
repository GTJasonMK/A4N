use crate::chapter_visual_plan::DirectorScriptPlan;
use a4n_db::{
    chapter_visual_plan::ParsedChapterVisualPlan, shooting_script::ParsedChapterShootingScript,
};
use a4n_domain::source_spans::SourceSpan;
use a4n_domain::{
    chapter::ResolvedChapterSource,
    shooting_script::{
        ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingTransitionContractV1,
    },
};
use a4n_error::AppError;
use serde::{Deserialize, Serialize};

pub(super) const CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT: &str = "chapter_screenplay_layout_unit";
pub(super) const CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT: &str =
    "chapter_screenplay_unit_adaptation";
pub(super) const CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT: &str = "chapter_shooting_design_unit";
pub(super) const CHAPTER_SHOOTING_SCRIPT_WORKFLOW: &str = "chapter_shooting_script";
pub(super) const CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL: u32 = 100;
pub(super) type ShootingScriptProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);
pub(super) type ShootingScriptStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

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
pub(super) struct ScreenplayLayoutUnitDetailV2 {
    pub(super) schema_version: String,
    pub(super) narrative_unit_index: u32,
    pub(super) source_span_ids: Vec<String>,
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
    /// 将表演事实绑定到 confirmed visual plan 的唯一镜头，供镜头级生成和校验使用。
    pub(super) shot_id: String,
    pub(super) source_span_ids: Vec<String>,
    #[serde(default)]
    pub(super) user_addition_ids: Vec<String>,
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
    pub(super) shot_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedChapterShootingScript {
    pub source: ResolvedChapterSource,
    pub shooting_script: ParsedChapterShootingScript,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedChapterShootingScriptRequest {
    pub(super) source: ResolvedChapterSource,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) input_fingerprint_sha256: String,
    pub(super) existing: Option<ParsedChapterShootingScript>,
    pub(super) visual_plan: ParsedChapterVisualPlan,
    pub(super) visual_plan_projection: DirectorScriptPlan,
    pub(super) shot_ids: Vec<String>,
    pub(super) scene_profiles: Vec<serde_json::Value>,
    pub(super) layout_unit_system_prompt: String,
    pub(super) unit_adaptation_system_prompt: String,
    pub(super) design_unit_system_prompt: String,
}

impl PreparedChapterShootingScriptRequest {
    pub(super) fn visual_unit(
        &self,
        narrative_unit_index: u32,
        scene_identity: &str,
    ) -> Result<&crate::chapter_visual_plan::DirectorScriptUnit, AppError> {
        narrative_unit_index
            .checked_sub(1)
            .and_then(|index| {
                self.visual_plan_projection
                    .narrative_units
                    .get(index as usize)
            })
            .filter(|unit| unit.id == scene_identity)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "拍摄单元 {narrative_unit_index} 找不到对应的已确认导演编排"
                ))
            })
    }
}

#[cfg(test)]
pub(crate) fn visual_plan_fixture(source: &ResolvedChapterSource) -> ParsedChapterVisualPlan {
    use a4n_domain::chapter_visual_plan::{
        CHAPTER_VISUAL_PLAN_V3, ChapterVisualPlanAuthoringMode, ChapterVisualPlanStatus,
        ChapterVisualPlanV3, SceneBindingV3, VisualPointV3, VisualSegmentV3,
    };
    let mut plan = ChapterVisualPlanV3::starter();
    plan.points[1].at_ms = 8_000;
    plan.points.insert(
        1,
        VisualPointV3 {
            id: "middle".into(),
            at_ms: 4_000,
            label: "镜头边界".into(),
            image_id: None,
            description: "下一镜头的起始画面".into(),
            reference_image_ids: vec![],
        },
    );
    plan.segments[0].end_point_id = "middle".into();
    plan.segments[0].prompt = "推进当前行动".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: 1,
    });
    plan.segments.push(VisualSegmentV3 {
        id: "shot-2".into(),
        title: "镜头 2".into(),
        start_point_id: "middle".into(),
        end_point_id: "end".into(),
        prompt: "继续推进人物行动".into(),
        scene_binding: Some(SceneBindingV3::Scene {
            scene_profile_id: 1,
        }),
        use_end_point_image: false,
    });
    let plan_sha256 =
        a4n_db::generation::canonical_json_sha256(&plan).expect("visual plan fixture sha256");
    let now = chrono::Utc::now().naive_utc();
    ParsedChapterVisualPlan {
        edit_receipt: None,
        record: a4n_db_models::models::chapter::chapter_visual_plan::Model {
            id: 1,
            project_id: source.requested_project_id,
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.source_snapshot_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            source_revision_key: source.source_revision_key.clone(),
            source_content_sha256: source.content_sha256.clone(),
            contract_version: CHAPTER_VISUAL_PLAN_V3.into(),
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

#[cfg(test)]
pub(crate) async fn seed_visual_plan_scene(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> i64 {
    if let Some(scene) = a4n_db::scene_profile::maybe_scene_profile_by_normalized_name(
        db,
        project_id,
        "director_fixture",
    )
    .await
    .unwrap()
    {
        return scene.id;
    }
    a4n_db::scene_profile::create_scene_profile_record(
        db,
        project_id,
        a4n_domain::scene_profile::SceneProfileRecordInput {
            name: "导演场景".into(),
            normalized_name: "director_fixture".into(),
            aliases_json: serde_json::json!([]),
            scene_type: "building".into(),
            importance: "primary".into(),
            parent_scene_id: None,
            base_description: "夜间的宫殿".into(),
            visual_description: "门与走廊形成清晰空间关系".into(),
            spatial_layout: "门口通向走廊".into(),
            atmosphere: "安静".into(),
            entry_points_json: serde_json::json!([]),
            rules_json: serde_json::json!([]),
            first_seen_chapter: 1,
            last_seen_chapter: 1,
            evidence_json: serde_json::json!([]),
            image_prompt_seed: "夜间建筑".into(),
        },
        a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER,
    )
    .await
    .unwrap()
    .id
}
