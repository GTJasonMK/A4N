use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::event::sse::{ProgressMeta, ProgressPhaseKind, ProgressTaskKind};
use a4n_engine::llm::provider::{
    ChatMessage, ChatRequest, LlmProvider, LlmTraceContext, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WorkflowProgressEventInput, WorkflowRunService,
};

use super::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgameCharacterStaging, GalgamePipelinePromptChapter,
    GalgamePipelinePromptSourceSpan, GalgamePipelineScene, GalgamePipelineSourceSpan,
    GalgamePipelineStartStage, GalgameScriptAdaptation, GalgameScriptDetail, GalgameService,
    GalgameVisualPlan, branch_design_from_value, build_galgame_script_persist_plan,
    character_staging_from_value, galgame_branch_design_user_message,
    galgame_character_staging_user_message, galgame_scene_segmentation_user_message,
    galgame_script_adaptation_user_message, galgame_visual_planning_user_message,
    parse_galgame_json_object, scene_segmentation_from_value, script_adaptation_from_value,
    should_restore_galgame_stage_checkpoint, visual_plan_from_value,
};

const GALGAME_PIPELINE_WORKFLOW: &str = "galgame_pipeline";
const GALGAME_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
const CHECKPOINT_SCENES: &str = "scenes";
const CHECKPOINT_LINES_DRAFT: &str = "lines_draft";
const CHECKPOINT_LINES_STAGED: &str = "lines_staged";
const CHECKPOINT_VISUAL_PLAN: &str = "visual_plan";
const CHECKPOINT_BRANCH_PLAN: &str = "branch_plan";
const CHECKPOINT_PERSIST_SCRIPT: &str = "persist_script";
const CHECKPOINT_ASSET_GENERATION: &str = "asset_generation";
const SOURCE_SYNC_STAGE_SCENE_SEGMENTATION: &str = "scene_segmentation";
const SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION: &str = "script_adaptation";
const SOURCE_SYNC_STAGE_CHARACTER_STAGING: &str = "character_staging";
const SOURCE_SYNC_STAGE_VISUAL_PLANNING: &str = "visual_planning";
const SOURCE_SYNC_STAGE_BRANCH_DESIGN: &str = "branch_design";
const SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS: &[&str] = &[
    SOURCE_SYNC_STAGE_SCENE_SEGMENTATION,
    SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION,
    SOURCE_SYNC_STAGE_CHARACTER_STAGING,
    SOURCE_SYNC_STAGE_VISUAL_PLANNING,
    SOURCE_SYNC_STAGE_BRANCH_DESIGN,
];
const ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS: &[&str] = &[
    CHECKPOINT_SCENES,
    CHECKPOINT_LINES_DRAFT,
    CHECKPOINT_LINES_STAGED,
    CHECKPOINT_VISUAL_PLAN,
    CHECKPOINT_BRANCH_PLAN,
    CHECKPOINT_PERSIST_SCRIPT,
    CHECKPOINT_ASSET_GENERATION,
];

struct GalgameLlmStageInput<'a> {
    workflow: &'a WorkflowRunService,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    root_project_id: i64,
    workflow_run_id: i64,
    stage: GalgamePipelineStartStage,
    checkpoint_key: &'a str,
    prompt_name: &'a str,
    user_prompt: String,
    restore_from_checkpoint: bool,
    restore_required: bool,
}

struct GalgameLlmOnceInput<'a> {
    service: &'a GalgameService,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    root_project_id: i64,
    workflow_run_id: i64,
    workflow: &'a str,
    step_key: String,
    call_order: i32,
    prompt_name: &'a str,
    user_prompt: String,
}

pub type GalgameProgressReporter =
    Arc<dyn Fn(GalgamePipelineProgressUpdate) -> Result<(), AppError> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct GalgamePipelineProgressUpdate {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub meta: ProgressMeta,
}

#[derive(Clone)]
pub struct GalgamePipelineRunOptions {
    pub workflow_run_id: i64,
    pub task_id: Option<String>,
    pub progress_reporter: Option<GalgameProgressReporter>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<GalgamePipelineStartStage>,
}

#[derive(Debug, Clone)]
pub struct GalgameSceneRegenerationInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub workflow_run_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub chapter_title: Option<String>,
    pub chapter_summary: Option<String>,
    pub chapter_content_hash: String,
    pub chapter_content: String,
    pub scene_id: i64,
    pub scene_index: i32,
    pub scene_title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub background_note: Option<String>,
    pub source_spans: Vec<GalgamePipelinePromptSourceSpan>,
    pub speaker_profiles: Vec<super::GalgameSpeakerProfile>,
}

#[derive(Debug, Clone)]
pub struct GalgameSceneRegenerationPlan {
    pub scene: super::GalgamePersistScene,
    pub stage_outputs_json: Value,
}

impl GalgameService {
    pub async fn run_pipeline(
        &self,
        project_id: i64,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        options: GalgamePipelineRunOptions,
    ) -> Result<GalgameScriptDetail, AppError> {
        let context = self.load_pipeline_prompt_context(project_id).await?;
        let workflow = WorkflowRunService::new(self.db.clone());
        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;

        if !options.resume_from_checkpoint && options.start_from_stage.is_none() {
            workflow
                .delete_checkpoints_by_keys(
                    options.workflow_run_id,
                    ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS,
                )
                .await?;
        }
        if let Some(start_stage) = options.start_from_stage {
            workflow
                .delete_checkpoints_by_keys(
                    options.workflow_run_id,
                    start_stage.checkpoint_keys_to_clear(),
                )
                .await?;
        }

        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::SceneSegmentation,
            1,
            GalgamePipelineStartStage::SceneSegmentation.message(),
        )
        .await?;

        let should_restore = |stage: GalgamePipelineStartStage| {
            should_restore_galgame_stage_checkpoint(
                options.resume_from_checkpoint,
                options.start_from_stage,
                stage.order(),
            )
        };
        let restore_required = |stage: GalgamePipelineStartStage| {
            options
                .start_from_stage
                .is_some_and(|start| stage.order() < start.order())
        };

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        let scene_stage = GalgamePipelineStartStage::SceneSegmentation;
        let scene_user = galgame_scene_segmentation_user_message(&context.chapters)?;
        let scenes_json = self
            .run_galgame_llm_stage(
                GalgameLlmStageInput {
                    workflow: &workflow,
                    llm,
                    prompt_engine,
                    project_id,
                    root_project_id: context.root_project_id,
                    workflow_run_id: options.workflow_run_id,
                    stage: scene_stage,
                    checkpoint_key: CHECKPOINT_SCENES,
                    prompt_name: "galgame_scene_segmentation",
                    user_prompt: scene_user,
                    restore_from_checkpoint: should_restore(scene_stage),
                    restore_required: restore_required(scene_stage),
                },
                |value| {
                    let parsed =
                        scene_segmentation_from_value(value, "galgame_scene_segmentation")?;
                    serde_json::to_value(parsed).map_err(AppError::Serialization)
                },
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::SceneSegmentation,
            1,
            "已完成 Galgame 场景切分",
        )
        .await?;
        let scenes = scene_segmentation_from_value(scenes_json.clone(), CHECKPOINT_SCENES)?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::ScriptAdaptation,
            2,
            GalgamePipelineStartStage::ScriptAdaptation.message(),
        )
        .await?;
        let script_stage = GalgamePipelineStartStage::ScriptAdaptation;
        let script_user = galgame_script_adaptation_user_message(&context.chapters, &scenes_json)?;
        let lines_draft_json = self
            .run_galgame_llm_stage(
                GalgameLlmStageInput {
                    workflow: &workflow,
                    llm,
                    prompt_engine,
                    project_id,
                    root_project_id: context.root_project_id,
                    workflow_run_id: options.workflow_run_id,
                    stage: script_stage,
                    checkpoint_key: CHECKPOINT_LINES_DRAFT,
                    prompt_name: "galgame_script_adaptation",
                    user_prompt: script_user,
                    restore_from_checkpoint: should_restore(script_stage),
                    restore_required: restore_required(script_stage),
                },
                |value| {
                    let parsed = script_adaptation_from_value(value, "galgame_script_adaptation")?;
                    serde_json::to_value(parsed).map_err(AppError::Serialization)
                },
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::ScriptAdaptation,
            2,
            "已完成 Galgame 剧本行改编",
        )
        .await?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::CharacterStaging,
            3,
            GalgamePipelineStartStage::CharacterStaging.message(),
        )
        .await?;
        let staging_stage = GalgamePipelineStartStage::CharacterStaging;
        let staging_user =
            galgame_character_staging_user_message(&lines_draft_json, &context.speaker_profiles)?;
        let lines_staged_json = self
            .run_galgame_llm_stage(
                GalgameLlmStageInput {
                    workflow: &workflow,
                    llm,
                    prompt_engine,
                    project_id,
                    root_project_id: context.root_project_id,
                    workflow_run_id: options.workflow_run_id,
                    stage: staging_stage,
                    checkpoint_key: CHECKPOINT_LINES_STAGED,
                    prompt_name: "galgame_character_staging",
                    user_prompt: staging_user,
                    restore_from_checkpoint: should_restore(staging_stage),
                    restore_required: restore_required(staging_stage),
                },
                |value| {
                    let parsed = character_staging_from_value(value, "galgame_character_staging")?;
                    serde_json::to_value(parsed).map_err(AppError::Serialization)
                },
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::CharacterStaging,
            3,
            "已完成 Galgame 角色演出标注",
        )
        .await?;
        let staging =
            character_staging_from_value(lines_staged_json.clone(), CHECKPOINT_LINES_STAGED)?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::VisualPlanning,
            4,
            GalgamePipelineStartStage::VisualPlanning.message(),
        )
        .await?;
        let visual_stage = GalgamePipelineStartStage::VisualPlanning;
        let visual_user = galgame_visual_planning_user_message(&scenes_json, &lines_staged_json)?;
        let visual_plan_json = self
            .run_galgame_llm_stage(
                GalgameLlmStageInput {
                    workflow: &workflow,
                    llm,
                    prompt_engine,
                    project_id,
                    root_project_id: context.root_project_id,
                    workflow_run_id: options.workflow_run_id,
                    stage: visual_stage,
                    checkpoint_key: CHECKPOINT_VISUAL_PLAN,
                    prompt_name: "galgame_visual_planning",
                    user_prompt: visual_user,
                    restore_from_checkpoint: should_restore(visual_stage),
                    restore_required: restore_required(visual_stage),
                },
                |value| {
                    let parsed = visual_plan_from_value(value, "galgame_visual_planning")?;
                    serde_json::to_value(parsed).map_err(AppError::Serialization)
                },
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::VisualPlanning,
            4,
            "已完成 Galgame 视觉资产规划",
        )
        .await?;
        let visual_plan = visual_plan_from_value(visual_plan_json.clone(), CHECKPOINT_VISUAL_PLAN)?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::BranchDesign,
            5,
            GalgamePipelineStartStage::BranchDesign.message(),
        )
        .await?;
        let branch_stage = GalgamePipelineStartStage::BranchDesign;
        let branch_user = galgame_branch_design_user_message(&scenes_json, &lines_staged_json)?;
        let branch_plan_json = self
            .run_galgame_llm_stage(
                GalgameLlmStageInput {
                    workflow: &workflow,
                    llm,
                    prompt_engine,
                    project_id,
                    root_project_id: context.root_project_id,
                    workflow_run_id: options.workflow_run_id,
                    stage: branch_stage,
                    checkpoint_key: CHECKPOINT_BRANCH_PLAN,
                    prompt_name: "galgame_branch_design",
                    user_prompt: branch_user,
                    restore_from_checkpoint: should_restore(branch_stage),
                    restore_required: restore_required(branch_stage),
                },
                |value| {
                    let parsed = branch_design_from_value(value, "galgame_branch_design")?;
                    serde_json::to_value(parsed).map_err(AppError::Serialization)
                },
            )
            .await?;
        let branch_choice_count = branch_plan_json
            .get("choices")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let branch_plan =
            branch_design_from_value(branch_plan_json.clone(), CHECKPOINT_BRANCH_PLAN)?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::BranchDesign,
            5,
            "已完成 Galgame 分支设计",
        )
        .await?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        workflow
            .mark_current_step(
                options.workflow_run_id,
                GalgamePipelineStartStage::PersistScript.key(),
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::PersistScript,
            6,
            GalgamePipelineStartStage::PersistScript.message(),
        )
        .await?;
        let persist_plan = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &visual_plan,
            Some(&branch_plan),
            &context.source_spans,
        )?;
        let detail = self
            .persist_pipeline_script(project_id, persist_plan)
            .await?;
        let line_count = detail
            .scenes
            .iter()
            .map(|scene| scene.lines.len())
            .sum::<usize>();
        workflow
            .upsert_checkpoint(UpsertWorkflowCheckpointInput {
                workflow_run_id: options.workflow_run_id,
                step_key: CHECKPOINT_PERSIST_SCRIPT.to_string(),
                step_order: GalgamePipelineStartStage::PersistScript.order(),
                status: WORKFLOW_STATUS_COMPLETED.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: Some(json!({
                    "project_id": project_id,
                    "source_snapshot_id": context.source_snapshot_id,
                    "scenes_checkpoint": CHECKPOINT_SCENES,
                    "lines_staged_checkpoint": CHECKPOINT_LINES_STAGED,
                    "visual_plan_checkpoint": CHECKPOINT_VISUAL_PLAN,
                })),
                checkpoint_json: Some(json!({
                    "script_id": detail.script.id,
                    "scene_count": detail.scenes.len(),
                    "line_count": line_count,
                    "branch_choice_count": branch_choice_count,
                    "branch_design_persisted": branch_choice_count > 0,
                })),
                artifacts_json: Some(json!({
                    "galgame_script_id": detail.script.id,
                })),
                generation_run_ids_json: None,
                error_message: None,
                completed: true,
            })
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::PersistScript,
            6,
            "已写入 Galgame 剧本",
        )
        .await?;

        ensure_galgame_pipeline_not_cancelled(&workflow, options.workflow_run_id).await?;
        workflow
            .mark_current_step(
                options.workflow_run_id,
                GalgamePipelineStartStage::AssetGeneration.key(),
            )
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::AssetGeneration,
            7,
            GalgamePipelineStartStage::AssetGeneration.message(),
        )
        .await?;
        let asset_plan = self.plan_assets(project_id).await?;
        workflow
            .upsert_checkpoint(UpsertWorkflowCheckpointInput {
                workflow_run_id: options.workflow_run_id,
                step_key: CHECKPOINT_ASSET_GENERATION.to_string(),
                step_order: GalgamePipelineStartStage::AssetGeneration.order(),
                status: WORKFLOW_STATUS_COMPLETED.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: Some(json!({
                    "project_id": project_id,
                    "source_snapshot_id": context.source_snapshot_id,
                    "galgame_script_id": detail.script.id,
                })),
                checkpoint_json: Some(json!({
                    "background_specs": asset_plan.summary.background_specs,
                    "portrait_expression_specs": asset_plan.summary.portrait_expression_specs,
                    "event_cg_specs": asset_plan.summary.event_cg_specs,
                    "created_specs": asset_plan.summary.created_specs,
                    "updated_specs": asset_plan.summary.updated_specs,
                    "reused_specs": asset_plan.summary.reused_specs,
                })),
                artifacts_json: Some(json!({
                    "galgame_script_id": detail.script.id,
                    "generation_spec_count": asset_plan.summary.background_specs
                        + asset_plan.summary.portrait_expression_specs
                        + asset_plan.summary.event_cg_specs,
                })),
                generation_run_ids_json: None,
                error_message: None,
                completed: true,
            })
            .await?;
        self.report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::AssetGeneration,
            7,
            "已创建 Galgame 资产生成意图",
        )
        .await?;

        self.get_script_detail(project_id).await
    }

    pub async fn build_source_sync_scene_regeneration(
        &self,
        input: GalgameSceneRegenerationInput,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
    ) -> Result<GalgameSceneRegenerationPlan, AppError> {
        if input.source_spans.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 场 {} 局部重跑缺少目标源锚点",
                input.scene_id
            )));
        }
        let prompt_chapter = GalgamePipelinePromptChapter {
            source_snapshot_chapter_id: input.source_snapshot_chapter_id,
            source_chapter_id: input.source_chapter_id,
            chapter_number: input.chapter_number,
            title: input.chapter_title.clone(),
            summary: input.chapter_summary.clone(),
            content_hash: input.chapter_content_hash.clone(),
            content: input.chapter_content.clone(),
            source_spans: input.source_spans.clone(),
        };
        let persist_source_spans = input
            .source_spans
            .iter()
            .map(|span| GalgamePipelineSourceSpan {
                id: span.id.clone(),
                text_anchor_id: span.text_anchor_id,
                source_snapshot_chapter_id: span.source_snapshot_chapter_id,
            })
            .collect::<Vec<_>>();
        let scene_segmentation = super::GalgameSceneSegmentation {
            scenes: vec![GalgamePipelineScene {
                index: 1,
                title: input.scene_title.clone(),
                location: input.location.clone(),
                time_of_day: input.time_of_day.clone(),
                participants: Vec::new(),
                summary: input.background_note.clone(),
                source_span_ids: input
                    .source_spans
                    .iter()
                    .map(|span| span.id.clone())
                    .collect(),
            }],
        };
        let scenes_json =
            serde_json::to_value(&scene_segmentation).map_err(AppError::Serialization)?;

        let lines_draft_json = run_galgame_llm_once(
            GalgameLlmOnceInput {
                service: self,
                llm,
                prompt_engine,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.script_adaptation", input.scene_id),
                call_order: 1,
                prompt_name: "galgame_script_adaptation",
                user_prompt: galgame_script_adaptation_user_message(
                    std::slice::from_ref(&prompt_chapter),
                    &scenes_json,
                )?,
            },
            |value| {
                let parsed = script_adaptation_from_value(value, "galgame_script_adaptation")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
        let script_adaptation =
            script_adaptation_from_value(lines_draft_json.clone(), "lines_draft")?;
        ensure_single_regenerated_scene_lines(&script_adaptation)?;

        let lines_staged_json = run_galgame_llm_once(
            GalgameLlmOnceInput {
                service: self,
                llm,
                prompt_engine,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.character_staging", input.scene_id),
                call_order: 2,
                prompt_name: "galgame_character_staging",
                user_prompt: galgame_character_staging_user_message(
                    &lines_draft_json,
                    &input.speaker_profiles,
                )?,
            },
            |value| {
                let parsed = character_staging_from_value(value, "galgame_character_staging")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
        let staging = character_staging_from_value(lines_staged_json.clone(), "lines_staged")?;
        ensure_single_regenerated_staged_lines(&staging)?;

        let visual_plan_json = run_galgame_llm_once(
            GalgameLlmOnceInput {
                service: self,
                llm,
                prompt_engine,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.visual_planning", input.scene_id),
                call_order: 3,
                prompt_name: "galgame_visual_planning",
                user_prompt: galgame_visual_planning_user_message(
                    &scenes_json,
                    &lines_staged_json,
                )?,
            },
            |value| {
                let parsed = visual_plan_from_value(value, "galgame_visual_planning")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
        let visual_plan = visual_plan_from_value(visual_plan_json.clone(), "visual_plan")?;
        let visual_plan = normalize_single_scene_visual_plan(visual_plan)?;

        let branch_plan_json = run_galgame_llm_once(
            GalgameLlmOnceInput {
                service: self,
                llm,
                prompt_engine,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.branch_design", input.scene_id),
                call_order: 4,
                prompt_name: "galgame_branch_design",
                user_prompt: galgame_branch_design_user_message(&scenes_json, &lines_staged_json)?,
            },
            |value| {
                let parsed = branch_design_from_value(value, "galgame_branch_design")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
        let branch_plan = branch_design_from_value(branch_plan_json.clone(), "branch_plan")?;
        if !branch_plan.choices.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame source sync 场级 LLM 重跑暂不接受新增选择支；请先人工保留本地或重新运行完整 Galgame 管线"
                    .to_string(),
            ));
        }

        let persist_plan = build_galgame_script_persist_plan(
            &scene_segmentation,
            &staging,
            &visual_plan,
            Some(&branch_plan),
            &persist_source_spans,
        )?;
        let mut scene =
            persist_plan
                .scenes
                .into_iter()
                .next()
                .ok_or_else(|| AppError::InvalidState {
                    current: "galgame_source_sync_regeneration_empty_plan".to_string(),
                    operation: "build_source_sync_scene_regeneration".to_string(),
                })?;
        scene.scene_index = input.scene_index;

        Ok(GalgameSceneRegenerationPlan {
            scene,
            stage_outputs_json: json!({
                "scene_segmentation": scenes_json,
                "script_adaptation": lines_draft_json,
                "character_staging": lines_staged_json,
                "visual_planning": visual_plan_json,
                "branch_design": branch_plan_json,
            }),
        })
    }

    async fn run_galgame_llm_stage<F>(
        &self,
        input: GalgameLlmStageInput<'_>,
        validate: F,
    ) -> Result<Value, AppError>
    where
        F: Fn(Value) -> Result<Value, AppError>,
    {
        let GalgameLlmStageInput {
            workflow,
            llm,
            prompt_engine,
            project_id,
            root_project_id,
            workflow_run_id,
            stage,
            checkpoint_key,
            prompt_name,
            user_prompt,
            restore_from_checkpoint,
            restore_required,
        } = input;
        ensure_galgame_pipeline_not_cancelled(workflow, workflow_run_id).await?;
        workflow
            .mark_current_step(workflow_run_id, stage.key())
            .await?;
        let system_prompt = prompt_engine.render(prompt_name, &HashMap::new(), Some(project_id))?;
        let request_messages = vec![
            ChatMessage::system(system_prompt.clone()),
            ChatMessage::user(user_prompt.clone()),
        ];
        let fingerprint_json = json!({
            "workflow": GALGAME_PIPELINE_WORKFLOW,
            "stage": stage.key(),
            "checkpoint_key": checkpoint_key,
            "prompt_name": prompt_name,
            "system_prompt": system_prompt,
            "user_prompt": user_prompt,
            "temperature": 0.25,
            "max_tokens_profile": GALGAME_PIPELINE_MAX_TOKENS_PROFILE,
        });
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if restore_from_checkpoint {
            if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                self.db.clone(),
                workflow_run_id,
                checkpoint_key,
                &fingerprint_json,
            )
            .await?
            {
                let checkpoint_json =
                    required_llm_call_checkpoint_json(checkpoint, checkpoint_key)?;
                return restored_galgame_llm_stage_output(
                    &checkpoint_json,
                    checkpoint_key,
                    prompt_name,
                    validate,
                );
            }
            if restore_required {
                return Err(AppError::InvalidState {
                    current: format!("{checkpoint_key}_checkpoint_missing"),
                    operation: format!("从 {} 阶段重启 Galgame 管线", stage.key()),
                });
            }
        }

        let request = ChatRequest::new(
            request_messages,
            Some(0.25),
            None,
            Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
        )
        .with_json_response_format()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: Some(root_project_id),
            workflow: GALGAME_PIPELINE_WORKFLOW.to_string(),
            step_key: stage.key().to_string(),
            generation_run_id: None,
            workflow_run_id: Some(workflow_run_id),
            call_key: Some(checkpoint_key.to_string()),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        let response = llm.chat(request).await?;
        ensure_galgame_pipeline_not_cancelled(workflow, workflow_run_id).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidState {
                current: format!("{prompt_name}_response_truncated"),
                operation: "执行 Galgame 管线 LLM 阶段".to_string(),
            });
        }
        let raw_output = response.message.content;
        let raw_json = parse_galgame_json_object(&raw_output, prompt_name)?;
        let parsed_output = validate(raw_json)?;
        let usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: checkpoint_key.to_string(),
                call_order: stage.order(),
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output.clone(),
                    "parsed_output": parsed_output,
                    "response_content": raw_output,
                    "usage": usage_json,
                    "finish_reason": response.finish_reason,
                }),
                artifacts_json: Some(json!({
                    "stage": stage.key(),
                    "prompt_name": prompt_name,
                })),
            },
        )
        .await?;
        Ok(parsed_output)
    }

    async fn report_pipeline_progress(
        &self,
        workflow_run_id: i64,
        task_id: Option<&str>,
        progress_reporter: Option<&GalgameProgressReporter>,
        stage: GalgamePipelineStartStage,
        current: i32,
        message: &str,
    ) -> Result<(), AppError> {
        let workflow = WorkflowRunService::new(self.db.clone());
        ensure_galgame_pipeline_not_cancelled(&workflow, workflow_run_id).await?;
        let update = galgame_progress_update(stage, current, message);
        workflow
            .record_progress_event(WorkflowProgressEventInput {
                workflow_run_id,
                task_id,
                current: update.current,
                total: update.total,
                message,
                meta: Some(update.meta.clone()),
            })
            .await?;
        if let Some(reporter) = progress_reporter {
            reporter(update)?;
        }
        Ok(())
    }
}

fn galgame_progress_update(
    stage: GalgamePipelineStartStage,
    current: i32,
    message: &str,
) -> GalgamePipelineProgressUpdate {
    GalgamePipelineProgressUpdate {
        current: current.max(1) as u32,
        total: GALGAME_PIPELINE_TOTAL_STEPS as u32,
        message: message.to_string(),
        meta: galgame_progress_meta(stage, message),
    }
}

fn galgame_progress_meta(stage: GalgamePipelineStartStage, message: &str) -> ProgressMeta {
    let phase = match stage {
        GalgamePipelineStartStage::SceneSegmentation
        | GalgamePipelineStartStage::ScriptAdaptation
        | GalgamePipelineStartStage::CharacterStaging
        | GalgamePipelineStartStage::VisualPlanning
        | GalgamePipelineStartStage::BranchDesign => ProgressPhaseKind::ModelWait,
        GalgamePipelineStartStage::PersistScript => ProgressPhaseKind::Save,
        GalgamePipelineStartStage::AssetGeneration => ProgressPhaseKind::AssetGeneration,
    };
    ProgressMeta::new(
        ProgressTaskKind::GalgameGeneration,
        stage.key(),
        galgame_stage_label(stage),
        phase,
    )
    .with_detail(message)
}

fn galgame_stage_label(stage: GalgamePipelineStartStage) -> &'static str {
    match stage {
        GalgamePipelineStartStage::SceneSegmentation => "场景切分",
        GalgamePipelineStartStage::ScriptAdaptation => "剧本改编",
        GalgamePipelineStartStage::CharacterStaging => "角色演出",
        GalgamePipelineStartStage::VisualPlanning => "视觉规划",
        GalgamePipelineStartStage::BranchDesign => "分支设计",
        GalgamePipelineStartStage::PersistScript => "写入剧本",
        GalgamePipelineStartStage::AssetGeneration => "资产意图",
    }
}

async fn ensure_galgame_pipeline_not_cancelled(
    workflow: &WorkflowRunService,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    let run = workflow.get_run(workflow_run_id).await?;
    if run.status == WORKFLOW_STATUS_CANCELLED {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

async fn run_galgame_llm_once<F>(
    input: GalgameLlmOnceInput<'_>,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let GalgameLlmOnceInput {
        service,
        llm,
        prompt_engine,
        project_id,
        root_project_id,
        workflow_run_id,
        workflow,
        step_key,
        call_order,
        prompt_name,
        user_prompt,
    } = input;
    let system_prompt = prompt_engine.render(prompt_name, &HashMap::new(), Some(project_id))?;
    let fingerprint_json = json!({
        "workflow": workflow,
        "step_key": step_key.as_str(),
        "prompt_name": prompt_name,
        "system_prompt": system_prompt,
        "user_prompt": user_prompt.as_str(),
        "temperature": 0.25,
        "max_tokens_profile": GALGAME_PIPELINE_MAX_TOKENS_PROFILE,
    });
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
    WorkflowRunService::new(service.db.clone())
        .mark_current_step(workflow_run_id, &step_key)
        .await?;
    if let Some(checkpoint) = load_completed_llm_call_checkpoint(
        service.db.clone(),
        workflow_run_id,
        &step_key,
        &fingerprint_json,
    )
    .await?
    {
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &step_key)?;
        return restored_galgame_llm_stage_output(
            &checkpoint_json,
            &step_key,
            prompt_name,
            validate,
        );
    }
    let request = ChatRequest::new(
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        Some(0.25),
        None,
        Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
    )
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: Some(root_project_id),
        workflow: workflow.to_string(),
        step_key: step_key.clone(),
        generation_run_id: None,
        workflow_run_id: Some(workflow_run_id),
        call_key: Some(step_key.clone()),
        input_fingerprint_sha256: Some(input_fingerprint_sha256),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    let response = llm.chat(request).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidState {
            current: format!("{prompt_name}_response_truncated"),
            operation: "执行 Galgame source sync 局部 LLM 重跑".to_string(),
        });
    }
    let raw_output = response.message.content;
    let raw_json = parse_galgame_json_object(&raw_output, prompt_name)?;
    let parsed_output = validate(raw_json)?;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    save_completed_llm_call_checkpoint(
        service.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: step_key,
            call_order,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": parsed_output.clone(),
                "parsed_output": parsed_output.clone(),
                "response_content": raw_output,
                "usage": usage_json,
                "finish_reason": response.finish_reason,
            }),
            artifacts_json: Some(json!({
                "workflow": workflow,
                "prompt_name": prompt_name,
            })),
        },
    )
    .await?;
    Ok(parsed_output)
}

fn ensure_single_regenerated_scene_lines(
    script_adaptation: &GalgameScriptAdaptation,
) -> Result<(), AppError> {
    if script_adaptation.scene_lines.len() != 1
        || script_adaptation
            .scene_lines
            .first()
            .is_none_or(|scene| scene.scene_index != 1 || scene.lines.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回 scene_index=1 的非空剧本行".to_string(),
        ));
    }
    Ok(())
}

fn ensure_single_regenerated_staged_lines(
    character_staging: &GalgameCharacterStaging,
) -> Result<(), AppError> {
    if character_staging.scene_lines.len() != 1
        || character_staging
            .scene_lines
            .first()
            .is_none_or(|scene| scene.scene_index != 1 || scene.lines.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回 scene_index=1 的非空角色舞台行"
                .to_string(),
        ));
    }
    Ok(())
}

fn normalize_single_scene_visual_plan(
    visual_plan: GalgameVisualPlan,
) -> Result<GalgameVisualPlan, AppError> {
    if visual_plan.backgrounds.len() != 1 {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回一个 scene_index=1 背景规划".to_string(),
        ));
    }
    if visual_plan.backgrounds[0].scene_index != 1 {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑背景规划必须只引用 scene_index=1".to_string(),
        ));
    }
    if let Some(event_cg) = visual_plan
        .event_cgs
        .iter()
        .find(|item| item.scene_index != 1)
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 局部重跑事件 CG 不能引用其他场: scene_index={}",
            event_cg.scene_index
        )));
    }
    Ok(visual_plan)
}

pub(crate) fn validate_source_sync_scene_regeneration_stage_outputs(
    stage_outputs_json: &Value,
) -> Result<(), AppError> {
    let object = stage_outputs_json.as_object().ok_or_else(|| {
        AppError::InvalidArgument("Galgame source sync stage_outputs 必须是 JSON 对象".to_string())
    })?;
    for key in SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS {
        if !object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync stage_outputs 缺少 {key}"
            )));
        }
    }
    if let Some(key) = object
        .keys()
        .find(|key| !SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS.contains(&key.as_str()))
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync stage_outputs 包含未知阶段: {key}"
        )));
    }

    let scene_segmentation = scene_segmentation_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_SCENE_SEGMENTATION)?,
        "stage_outputs.scene_segmentation",
    )?;
    if scene_segmentation.scenes.len() != 1
        || scene_segmentation
            .scenes
            .first()
            .is_none_or(|scene| scene.index != 1 || scene.source_span_ids.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.scene_segmentation 必须只包含 scene_index=1 且带 source_span_ids 的单场"
                .to_string(),
        ));
    }

    let script_adaptation = script_adaptation_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION)?,
        "stage_outputs.script_adaptation",
    )?;
    ensure_single_regenerated_scene_lines(&script_adaptation)?;

    let character_staging = character_staging_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_CHARACTER_STAGING)?,
        "stage_outputs.character_staging",
    )?;
    ensure_single_regenerated_staged_lines(&character_staging)?;

    let visual_plan = visual_plan_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_VISUAL_PLANNING)?,
        "stage_outputs.visual_planning",
    )?;
    let visual_plan = normalize_single_scene_visual_plan(visual_plan)?;
    if visual_plan.backgrounds[0].source_span_ids.is_empty()
        || visual_plan
            .event_cgs
            .iter()
            .any(|event_cg| event_cg.source_span_ids.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.visual_planning 必须保留背景/事件 CG 的 source_span_ids"
                .to_string(),
        ));
    }

    let branch_design = branch_design_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_BRANCH_DESIGN)?,
        "stage_outputs.branch_design",
    )?;
    if !branch_design.choices.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.branch_design 不允许包含 choices".to_string(),
        ));
    }

    Ok(())
}

fn required_stage_output_value(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Value, AppError> {
    object
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame source sync stage_outputs.{key} 不能为空"))
        })
}

fn restored_galgame_llm_stage_output<F>(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    prompt_name: &str,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let checkpoint_parsed_output =
        required_checkpoint_value(checkpoint_json, checkpoint_key, "parsed_output")?;
    let response_content =
        required_checkpoint_string(checkpoint_json, checkpoint_key, "response_content")?;
    let reparsed_json = parse_galgame_json_object(&response_content, prompt_name)?;
    let reparsed_output = validate(reparsed_json)?;
    if reparsed_output != checkpoint_parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!(
                "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                checkpoint_key
            ),
        });
    }
    Ok(reparsed_output)
}

fn required_checkpoint_value(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    field: &str,
) -> Result<Value, AppError> {
    checkpoint_json
        .get(field)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{checkpoint_key}.{field}_missing"),
            operation: format!("读取 Galgame 管线 checkpoint {checkpoint_key}"),
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::galgame::{
        GalgameCharacterStaging, GalgamePipelineBackgroundPlan, GalgamePipelineEventCgPlan,
        GalgamePipelineLineDraft, GalgamePipelineLineStaged, GalgamePipelineSceneLineDraft,
        GalgamePipelineSceneLineStaged, LINE_TYPE_NARRATION,
    };

    use super::*;

    #[test]
    fn restored_galgame_llm_stage_output_revalidates_response_content() {
        let restored = restored_galgame_llm_stage_output(
            &json!({
                "parsed_output": { "value": 1 },
                "response_content": r#"{"value":1}"#
            }),
            "scenes",
            "galgame_test",
            Ok,
        )
        .expect("restored checkpoint");

        assert_eq!(restored, json!({ "value": 1 }));
    }

    #[test]
    fn restored_galgame_llm_stage_output_rejects_mismatched_parsed_output() {
        let err = restored_galgame_llm_stage_output(
            &json!({
                "parsed_output": { "value": 2 },
                "response_content": r#"{"value":1}"#
            }),
            "scenes",
            "galgame_test",
            Ok,
        )
        .expect_err("mismatched parsed_output must fail");

        assert!(
            err.to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }

    #[test]
    fn galgame_progress_update_uses_galgame_generation_meta_contract() {
        let update = galgame_progress_update(
            GalgamePipelineStartStage::PersistScript,
            0,
            "正在写入 Galgame 剧本",
        );
        let meta = serde_json::to_value(&update.meta).expect("progress meta json");

        assert_eq!(update.current, 1);
        assert_eq!(update.total, GALGAME_PIPELINE_TOTAL_STEPS as u32);
        assert_eq!(update.message, "正在写入 Galgame 剧本");
        assert_eq!(meta["taskKind"], "galgame_generation");
        assert_eq!(meta["stageCode"], "persist_script");
        assert_eq!(meta["stageLabel"], "写入剧本");
        assert_eq!(meta["phaseKind"], "save");
        assert_eq!(meta["detail"], "正在写入 Galgame 剧本");
    }

    #[test]
    fn source_sync_scene_regeneration_requires_one_non_empty_scene() {
        let empty = GalgameScriptAdaptation {
            scene_lines: vec![],
        };
        assert!(ensure_single_regenerated_scene_lines(&empty).is_err());

        let wrong_scene = GalgameScriptAdaptation {
            scene_lines: vec![GalgamePipelineSceneLineDraft {
                scene_index: 2,
                lines: vec![GalgamePipelineLineDraft {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_name: None,
                    text: "雨停了。".to_string(),
                    source_span_ids: vec!["ch1:ta1".to_string()],
                }],
            }],
        };
        assert!(ensure_single_regenerated_scene_lines(&wrong_scene).is_err());

        let valid = GalgameScriptAdaptation {
            scene_lines: vec![GalgamePipelineSceneLineDraft {
                scene_index: 1,
                lines: vec![GalgamePipelineLineDraft {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_name: None,
                    text: "雨停了。".to_string(),
                    source_span_ids: vec!["ch1:ta1".to_string()],
                }],
            }],
        };
        ensure_single_regenerated_scene_lines(&valid).expect("single non-empty scene should pass");
    }

    #[test]
    fn source_sync_scene_regeneration_requires_one_non_empty_staged_scene() {
        let empty = GalgameCharacterStaging {
            scene_lines: vec![],
        };
        assert!(ensure_single_regenerated_staged_lines(&empty).is_err());

        let wrong_scene = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 2,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    speaker_name: None,
                    text: "雨停了。".to_string(),
                    expression: None,
                    pose: None,
                    position: None,
                    source_span_ids: vec!["ch1:ta1".to_string()],
                }],
            }],
        };
        assert!(ensure_single_regenerated_staged_lines(&wrong_scene).is_err());

        let valid = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    speaker_name: None,
                    text: "雨停了。".to_string(),
                    expression: None,
                    pose: None,
                    position: None,
                    source_span_ids: vec!["ch1:ta1".to_string()],
                }],
            }],
        };
        ensure_single_regenerated_staged_lines(&valid)
            .expect("single non-empty staged scene should pass");
    }

    #[test]
    fn source_sync_scene_regeneration_visual_plan_rejects_extra_scene_backgrounds() {
        let plan = GalgameVisualPlan {
            backgrounds: vec![
                GalgamePipelineBackgroundPlan {
                    scene_index: 1,
                    background_note: "校门口雨后".to_string(),
                    scene_profile_id: None,
                    source_span_ids: vec!["ch1:ta1".to_string()],
                },
                GalgamePipelineBackgroundPlan {
                    scene_index: 2,
                    background_note: "旧场景".to_string(),
                    scene_profile_id: None,
                    source_span_ids: vec!["ch1:ta2".to_string()],
                },
            ],
            event_cgs: vec![],
        };

        let err =
            normalize_single_scene_visual_plan(plan).expect_err("extra scene must fail visibly");

        assert!(
            err.to_string()
                .contains("只返回一个 scene_index=1 背景规划")
        );
    }

    #[test]
    fn source_sync_scene_regeneration_visual_plan_rejects_extra_scene_event_cg() {
        let plan = GalgameVisualPlan {
            backgrounds: vec![GalgamePipelineBackgroundPlan {
                scene_index: 1,
                background_note: "校门口雨后".to_string(),
                scene_profile_id: None,
                source_span_ids: vec!["ch1:ta1".to_string()],
            }],
            event_cgs: vec![
                GalgamePipelineEventCgPlan {
                    scene_index: 1,
                    line_index: 1,
                    cg_note: "抬头看天".to_string(),
                    importance: "normal".to_string(),
                    source_span_ids: vec!["ch1:ta1".to_string()],
                },
                GalgamePipelineEventCgPlan {
                    scene_index: 2,
                    line_index: 1,
                    cg_note: "不能跨场".to_string(),
                    importance: "normal".to_string(),
                    source_span_ids: vec!["ch1:ta2".to_string()],
                },
            ],
        };

        let err =
            normalize_single_scene_visual_plan(plan).expect_err("extra event CG scene must fail");

        assert!(err.to_string().contains("事件 CG 不能引用其他场"));
    }

    #[test]
    fn source_sync_scene_regeneration_visual_plan_requires_background() {
        let plan = GalgameVisualPlan {
            backgrounds: vec![GalgamePipelineBackgroundPlan {
                scene_index: 2,
                background_note: "旧场景".to_string(),
                scene_profile_id: None,
                source_span_ids: vec!["ch1:ta2".to_string()],
            }],
            event_cgs: vec![],
        };

        assert!(normalize_single_scene_visual_plan(plan).is_err());
    }
}

fn required_checkpoint_string(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{checkpoint_key}.{field}_missing_or_invalid"),
            operation: format!("读取 Galgame 管线 checkpoint {checkpoint_key}"),
        })
}
