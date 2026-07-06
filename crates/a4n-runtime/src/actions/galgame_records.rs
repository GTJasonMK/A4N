use crate::AppState;
use crate::actions::access;
use crate::actions::export_files::ExportFileData;
use crate::actions::image_generation::{self, GenerateGalgameImageInput, GeneratedImageSummary};
use crate::actions::portrait_generation::{self, GeneratePortraitInput, PortraitSummary};
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::file_utils::{canonicalize_data_file_path, data_dir_relative_file_path};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::{novel_project, source_snapshot_chapter, source_snapshot_profile},
};
use a4n_engine::llm::provider::{ChatMessage, ChatRequest};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_services::galgame::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE,
    GALGAME_EXPRESSIONS, GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG,
    GALGAME_TARGET_PORTRAIT_EXPRESSION, GalgameAssetPlan, GalgamePipelineRunOptions,
    GalgamePipelineStartStage, GalgameProgressReporter, GalgameScriptDetail,
    InsertGalgameLineInput, LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE,
    LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION, PROJECT_KIND_GALGAME, ReorderGalgameLinesInput,
    SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN, UpdateGalgameChoicesInput, UpdateGalgameLineInput,
    UpdateGalgameSceneInput, branch_design_from_value, character_staging_from_value,
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_scene_segmentation_user_message, galgame_script_adaptation_user_message,
    galgame_visual_planning_user_message, parse_galgame_start_stage, scene_segmentation_from_value,
    script_adaptation_from_value, visual_plan_from_value,
};
use a4n_services::media::{
    CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_BINDING_ROLE_BACKGROUND,
    MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_CG, MEDIA_BINDING_ROLE_SFX, MEDIA_KIND_AUDIO,
    MEDIA_KIND_IMAGE, MEDIA_ORIGIN_GENERATED, MEDIA_ORIGIN_IMPORTED, MEDIA_STATUS_COMPLETED,
    galgame_media_binding_required_media_kind,
};
use a4n_services::workflow::{
    ListWorkflowRunsInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use base64::Engine as _;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Cursor;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";
const GALGAME_PIPELINE_MODE_FULL: &str = "full_pipeline_v1";
const GALGAME_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
const GALGAME_CHECKPOINT_SCENES: &str = "scenes";
const GALGAME_CHECKPOINT_LINES_DRAFT: &str = "lines_draft";
const GALGAME_CHECKPOINT_LINES_STAGED: &str = "lines_staged";
const GALGAME_CHECKPOINT_VISUAL_PLAN: &str = "visual_plan";
const GALGAME_CHECKPOINT_BRANCH_PLAN: &str = "branch_plan";
const MAX_IMPORT_AUDIO_BYTES: usize = 256 * 1024 * 1024;
const GALGAME_ASSET_RENDERER_VERSION: &str = "galgame_asset_generation_v1";
const GALGAME_PROMPT_KIND_BACKGROUND: &str = "background_image_prompt";
const GALGAME_PROMPT_KIND_EVENT_CG: &str = "event_cg_prompt";
const GALGAME_BACKGROUND_IMAGE_PROMPT_TEMPLATE: &str = "galgame_background_image_prompt";
const GALGAME_EVENT_CG_PROMPT_TEMPLATE: &str = "galgame_event_cg_prompt";
const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];
const GALGAME_PORTRAIT_VARIANT_STYLE: &str =
    "Galgame 视觉小说角色立绘表情差分；必须保持参考图中的角色身份、服装、体型比例和美术风格一致。";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalgameProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartGalgamePipelineInput {
    pub project_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub start_from_stage: Option<String>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgamePipelineStatusSummary {
    pub run: Option<GalgameWorkflowRunSummary>,
    pub checkpoints: Vec<GalgameWorkflowCheckpointSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameWorkflowRunSummary {
    pub id: i64,
    pub status: String,
    pub mode: Option<String>,
    pub current_step_key: Option<String>,
    pub progress_json: Option<Value>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameWorkflowCheckpointSummary {
    pub id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameMediaBindingResult {
    pub binding: media_binding::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindGalgameMediaInput {
    pub project_id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnbindGalgameMediaInput {
    pub project_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub media_asset_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportGalgameAudioInput {
    pub project_id: i64,
    pub target_line_id: i64,
    pub role: String,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunGalgameAssetGenerationInput {
    pub project_id: i64,
    #[serde(default)]
    pub generation_spec_ids: Vec<i64>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetGenerationSummary {
    pub project_id: i64,
    pub generated_count: usize,
    pub skipped_completed_count: usize,
    pub items: Vec<GalgameAssetGenerationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetGenerationItem {
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub status: String,
    pub prompt_artifact_id: Option<i64>,
    pub generated_image_id: Option<i64>,
    pub portrait_id: Option<i64>,
    pub media_asset_id: Option<i64>,
    pub binding_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteGalgameLineInput {
    pub line_id: i64,
}

pub async fn get_galgame_script(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameScriptDetail, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    state
        .galgame
        .get_script_detail(input.project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn start_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<GalgameScriptDetail, String> {
    start_galgame_pipeline_with_progress(state, user_id, input, None).await
}

pub async fn start_galgame_pipeline_with_progress(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
    progress_reporter: Option<GalgameProgressReporter>,
) -> Result<GalgameScriptDetail, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let start_from_stage = parse_galgame_start_stage(input.start_from_stage.as_deref())
        .map_err(|err| err.to_string())?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let fingerprint = json!({
        "project_id": project.id,
        "source_snapshot_id": source_snapshot_id,
        "root_project_id": root_project_id,
        "mode": GALGAME_PIPELINE_MODE_FULL,
    });
    let initial_stage = start_from_stage.unwrap_or(GalgamePipelineStartStage::SceneSegmentation);
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("galgame_pipeline:{}", project.id),
            workflow_kind: GALGAME_PIPELINE_WORKFLOW_KIND.to_string(),
            project_id: Some(project.id),
            root_project_id: Some(root_project_id),
            source_snapshot_id: Some(source_snapshot_id),
            scope_table: Some("galgame_script".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some(GALGAME_PIPELINE_MODE_FULL.to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(initial_stage.key().to_string()),
            active_task_id: input.task_id.clone(),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let result = state
        .galgame
        .run_pipeline(
            project.id,
            state.llm.as_ref(),
            state.engine.as_ref(),
            GalgamePipelineRunOptions {
                workflow_run_id: workflow_run.id,
                task_id: input.task_id.clone(),
                progress_reporter,
                resume_from_checkpoint: input.resume_from_checkpoint,
                start_from_stage,
            },
        )
        .await;
    match result {
        Ok(detail) => {
            state
                .workflow_run
                .mark_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .map_err(|err| err.to_string())?;
            Ok(detail)
        }
        Err(err) => {
            let cancelled = matches!(&err, AppError::Cancelled);
            let error_message = err.to_string();
            if cancelled {
                let _ = state
                    .workflow_run
                    .mark_status(
                        workflow_run.id,
                        WORKFLOW_STATUS_CANCELLED,
                        true,
                        Some("Galgame 管线已取消"),
                    )
                    .await;
                return Err(error_message);
            }
            let _ = state
                .workflow_run
                .mark_status(
                    workflow_run.id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(&error_message),
                )
                .await;
            Err(error_message)
        }
    }
}

pub async fn cancel_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!("项目不是 Galgame 派生项目: {}", project.id));
    }
    let Some(run) = state
        .workflow_run
        .list_runs(ListWorkflowRunsInput {
            project_id: Some(project.id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: Some(WORKFLOW_STATUS_RUNNING.to_string()),
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next()
    else {
        return Err("没有正在运行的 Galgame 管线可取消".to_string());
    };

    state
        .workflow_run
        .mark_status(
            run.id,
            WORKFLOW_STATUS_CANCELLED,
            true,
            Some("用户请求取消 Galgame 管线"),
        )
        .await
        .map_err(|err| err.to_string())?;
    get_galgame_pipeline_status(
        state,
        user_id,
        GalgameProjectInput {
            project_id: project.id,
        },
    )
    .await
}

pub async fn preview_galgame_pipeline_request(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let start_from_stage = parse_galgame_start_stage(input.start_from_stage.as_deref())
        .map_err(|err| err.to_string())?;
    let context = state
        .galgame
        .load_pipeline_prompt_context(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let checkpoint_inputs = load_galgame_preview_checkpoint_inputs(
        state,
        input.project_id,
        start_from_stage,
        input.resume_from_checkpoint,
    )
    .await?;
    let scene_segmentation_input = checkpoint_inputs.scene_segmentation_input()?;
    let script_adaptation_input = checkpoint_inputs.script_adaptation_input()?;
    let character_staging_input = checkpoint_inputs.character_staging_input()?;
    let mut steps = Vec::new();
    let stage_filter = |stage: GalgamePipelineStartStage| {
        start_from_stage
            .map(|start| stage.order() >= start.order())
            .unwrap_or(true)
    };

    if stage_filter(GalgamePipelineStartStage::SceneSegmentation) {
        steps.push(
            galgame_preview_step(
                state,
                "scene_segmentation",
                "场景切分",
                "galgame_scene_segmentation",
                galgame_scene_segmentation_user_message(&context.chapters)
                    .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::ScriptAdaptation)
        && let Some(scene_segmentation_input) = scene_segmentation_input.as_ref()
    {
        steps.push(
            galgame_preview_step(
                state,
                "script_adaptation",
                "剧本行改编",
                "galgame_script_adaptation",
                galgame_script_adaptation_user_message(&context.chapters, scene_segmentation_input)
                    .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::CharacterStaging)
        && let Some(script_adaptation_input) = script_adaptation_input.as_ref()
    {
        steps.push(
            galgame_preview_step(
                state,
                "character_staging",
                "角色演出标注",
                "galgame_character_staging",
                galgame_character_staging_user_message(
                    script_adaptation_input,
                    &context.speaker_profiles,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::VisualPlanning)
        && let (Some(scene_segmentation_input), Some(character_staging_input)) = (
            scene_segmentation_input.as_ref(),
            character_staging_input.as_ref(),
        )
    {
        steps.push(
            galgame_preview_step(
                state,
                "visual_planning",
                "视觉资产规划",
                "galgame_visual_planning",
                galgame_visual_planning_user_message(
                    scene_segmentation_input,
                    character_staging_input,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::BranchDesign)
        && let (Some(scene_segmentation_input), Some(character_staging_input)) = (
            scene_segmentation_input.as_ref(),
            character_staging_input.as_ref(),
        )
    {
        steps.push(
            galgame_preview_step(
                state,
                "branch_design",
                "分支设计",
                "galgame_branch_design",
                galgame_branch_design_user_message(
                    scene_segmentation_input,
                    character_staging_input,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }

    Ok(model_request_preview(
        "galgame_pipeline",
        "Galgame 七阶段管线请求预览",
        steps,
    ))
}

#[derive(Default)]
struct GalgamePreviewCheckpointInputs {
    scenes: Option<Value>,
    lines_draft: Option<Value>,
    lines_staged: Option<Value>,
    visual_plan: Option<Value>,
    branch_plan: Option<Value>,
    required_prior_stage: Option<GalgamePipelineStartStage>,
}

impl GalgamePreviewCheckpointInputs {
    fn scene_segmentation_input(&self) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::SceneSegmentation,
            self.scenes.clone(),
        )
    }

    fn script_adaptation_input(&self) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::ScriptAdaptation,
            self.lines_draft.clone(),
        )
    }

    fn character_staging_input(&self) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::CharacterStaging,
            self.lines_staged.clone(),
        )
    }

    fn prior_stage_input(
        &self,
        stage: GalgamePipelineStartStage,
        value: Option<Value>,
    ) -> Result<Option<Value>, String> {
        if self
            .required_prior_stage
            .is_some_and(|start| stage.order() < start.order())
        {
            return value.map(Some).ok_or_else(|| {
                format!(
                    "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                    start_stage_key(self.required_prior_stage),
                    stage.key()
                )
            });
        }
        Ok(value)
    }

    fn validate_required_prior_checkpoints(&self) -> Result<(), String> {
        let Some(start) = self.required_prior_stage else {
            return Ok(());
        };
        let required = [
            (GalgamePipelineStartStage::SceneSegmentation, &self.scenes),
            (
                GalgamePipelineStartStage::ScriptAdaptation,
                &self.lines_draft,
            ),
            (
                GalgamePipelineStartStage::CharacterStaging,
                &self.lines_staged,
            ),
            (GalgamePipelineStartStage::VisualPlanning, &self.visual_plan),
            (GalgamePipelineStartStage::BranchDesign, &self.branch_plan),
        ];
        for (stage, value) in required {
            if stage.order() < start.order() && value.is_none() {
                return Err(format!(
                    "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                    start.key(),
                    stage.key()
                ));
            }
        }
        Ok(())
    }
}

async fn load_galgame_preview_checkpoint_inputs(
    state: &AppState,
    project_id: i64,
    start_from_stage: Option<GalgamePipelineStartStage>,
    resume_from_checkpoint: bool,
) -> Result<GalgamePreviewCheckpointInputs, String> {
    let requires_prior_checkpoint = start_from_stage.is_some();
    if !requires_prior_checkpoint && !resume_from_checkpoint {
        return Ok(GalgamePreviewCheckpointInputs::default());
    }
    let mut inputs = GalgamePreviewCheckpointInputs {
        required_prior_stage: start_from_stage,
        ..Default::default()
    };
    let Some(run) = state
        .workflow_run
        .list_runs(ListWorkflowRunsInput {
            project_id: Some(project_id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: None,
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next()
    else {
        return Ok(inputs);
    };
    for checkpoint in state
        .workflow_run
        .checkpoints(run.id)
        .await
        .map_err(|err| err.to_string())?
    {
        if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
            continue;
        }
        match checkpoint.step_key.as_str() {
            GALGAME_CHECKPOINT_SCENES => {
                inputs.scenes = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_SCENES,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_LINES_DRAFT => {
                inputs.lines_draft = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_LINES_DRAFT,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_LINES_STAGED => {
                inputs.lines_staged = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_LINES_STAGED,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_VISUAL_PLAN => {
                inputs.visual_plan = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_VISUAL_PLAN,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_BRANCH_PLAN => {
                inputs.branch_plan = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_BRANCH_PLAN,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            other => {
                return Err(format!(
                    "Galgame 管线存在未知 completed checkpoint: {other}"
                ));
            }
        }
    }
    inputs.validate_required_prior_checkpoints()?;
    Ok(inputs)
}

fn galgame_preview_checkpoint_output(
    checkpoint_key: &str,
    checkpoint_json: Option<&Value>,
) -> Result<Value, String> {
    let object = checkpoint_json
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Galgame checkpoint {checkpoint_key} 缺少 JSON 对象"))?;
    let output = object
        .get("output")
        .ok_or_else(|| format!("Galgame checkpoint {checkpoint_key} 缺少 output"))?
        .clone();
    if let Some(parsed_output) = object.get("parsed_output")
        && parsed_output != &output
    {
        return Err(format!(
            "Galgame checkpoint {checkpoint_key} 的 output 与 parsed_output 不一致"
        ));
    }
    match checkpoint_key {
        GALGAME_CHECKPOINT_SCENES => {
            scene_segmentation_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_LINES_DRAFT => {
            script_adaptation_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_LINES_STAGED => {
            character_staging_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_VISUAL_PLAN => {
            visual_plan_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_BRANCH_PLAN => {
            branch_design_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        other => {
            return Err(format!("Galgame checkpoint key 不支持: {other}"));
        }
    }
    Ok(output)
}

fn start_stage_key(stage: Option<GalgamePipelineStartStage>) -> &'static str {
    stage
        .map(|stage| stage.key())
        .unwrap_or("scene_segmentation")
}

async fn galgame_preview_step(
    state: &AppState,
    key: &str,
    title: &str,
    prompt_name: &str,
    user_prompt: String,
) -> Result<crate::actions::request_preview::ModelRequestPreviewStep, String> {
    let system_prompt = state
        .engine
        .render(prompt_name, &std::collections::HashMap::new(), None)
        .map_err(|err| err.to_string())?;
    let request = ChatRequest::new(
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        Some(0.25),
        None,
        Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
    )
    .with_json_response_format();
    chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
}

pub async fn get_galgame_pipeline_status(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!("项目不是 Galgame 派生项目: {}", project.id));
    }
    let run = state
        .workflow_run
        .list_runs(ListWorkflowRunsInput {
            project_id: Some(project.id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: None,
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next();
    let Some(run) = run else {
        return Ok(GalgamePipelineStatusSummary {
            run: None,
            checkpoints: Vec::new(),
        });
    };
    let checkpoints = state
        .workflow_run
        .checkpoints(run.id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|checkpoint| GalgameWorkflowCheckpointSummary {
            id: checkpoint.id,
            step_key: checkpoint.step_key,
            step_order: checkpoint.step_order,
            status: checkpoint.status,
            checkpoint_json: checkpoint.checkpoint_json,
            artifacts_json: checkpoint.artifacts_json,
            error_message: checkpoint.error_message,
            created_at: checkpoint.created_at.to_string(),
            updated_at: checkpoint.updated_at.to_string(),
            completed_at: checkpoint.completed_at.map(|value| value.to_string()),
        })
        .collect();
    Ok(GalgamePipelineStatusSummary {
        run: Some(GalgameWorkflowRunSummary {
            id: run.id,
            status: run.status,
            mode: run.mode,
            current_step_key: run.current_step_key,
            progress_json: run.progress_json,
            active_task_id: run.active_task_id,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
            error_message: run.error_message,
            created_at: run.created_at.to_string(),
            updated_at: run.updated_at.to_string(),
            completed_at: run.completed_at.map(|value| value.to_string()),
        }),
        checkpoints,
    })
}

pub async fn update_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .update_line(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn insert_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: InsertGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .insert_line(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_scene(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameSceneInput,
) -> Result<a4n_core::models::galgame::galgame_scene::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .update_scene(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn delete_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: DeleteGalgameLineInput,
) -> Result<(), String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .delete_line(project_id, input.line_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn reorder_galgame_lines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ReorderGalgameLinesInput,
) -> Result<Vec<galgame_line::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .reorder_lines(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_choices(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameChoicesInput,
) -> Result<Vec<galgame_choice::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .galgame
        .update_choices(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn plan_galgame_assets(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameAssetPlan, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .galgame
        .plan_assets(input.project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn run_galgame_asset_generation(
    state: &AppState,
    user_id: i64,
    input: RunGalgameAssetGenerationInput,
) -> Result<GalgameAssetGenerationSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    validate_run_galgame_asset_generation_input(&input)?;

    let specs = load_galgame_asset_specs(state, &input).await?;
    let mut summary = GalgameAssetGenerationSummary {
        project_id: input.project_id,
        generated_count: 0,
        skipped_completed_count: 0,
        items: Vec::with_capacity(specs.len()),
    };
    for spec in specs {
        let (item, generated) = match spec.target_kind.as_str() {
            GALGAME_TARGET_BACKGROUND | GALGAME_TARGET_CG => {
                run_galgame_visual_spec_generation(state, user_id, &input, &spec).await?
            }
            GALGAME_TARGET_PORTRAIT_EXPRESSION => {
                run_galgame_portrait_spec_generation(state, user_id, &spec).await?
            }
            other => {
                return Err(format!(
                    "Galgame 资产生成不支持 target_kind: spec_id={}, target_kind={other}",
                    spec.id
                ));
            }
        };
        if generated {
            summary.generated_count += 1;
        } else {
            summary.skipped_completed_count += 1;
        }
        summary.items.push(item);
    }
    Ok(summary)
}

fn validate_run_galgame_asset_generation_input(
    input: &RunGalgameAssetGenerationInput,
) -> Result<(), String> {
    if let Some(limit) = input.limit
        && limit == 0
    {
        return Err("limit 必须大于 0".to_string());
    }
    let mut seen = BTreeSet::new();
    for spec_id in &input.generation_spec_ids {
        if *spec_id <= 0 {
            return Err("generation_spec_ids 必须全部是正整数".to_string());
        }
        if !seen.insert(*spec_id) {
            return Err(format!("generation_spec_ids 不能包含重复 ID: {spec_id}"));
        }
    }
    Ok(())
}

pub async fn bind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    bind_galgame_media_inner(state, input).await
}

pub async fn unbind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: UnbindGalgameMediaInput,
) -> Result<u64, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let target_kind = normalize_galgame_media_target_kind(&input.target_kind)?;
    let role = normalize_galgame_media_role(&target_kind, &input.role)?;
    validate_galgame_media_target(state, input.project_id, &target_kind, input.target_id).await?;
    if let Some(media_asset_id) = input.media_asset_id
        && media_asset_id <= 0
    {
        return Err("media_asset_id 必须是正整数".to_string());
    }
    let mut delete = media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(input.target_id))
        .filter(media_binding::Column::Role.eq(role));
    if let Some(media_asset_id) = input.media_asset_id {
        delete = delete.filter(media_binding::Column::MediaAssetId.eq(media_asset_id));
    }
    let result = delete
        .exec(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(result.rows_affected)
}

pub async fn import_galgame_audio(
    state: &AppState,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let role = normalize_audio_role(&input.role)?;
    let target_context = validate_galgame_media_target(
        state,
        input.project_id,
        GALGAME_ENTITY_LINE,
        input.target_line_id,
    )
    .await?;
    let source =
        ImportedAudioSource::from_upload(&input.file_name, &input.mime_type, &input.data_base64)?;
    let output = galgame_audio_output_path(state, input.project_id, source.extension)?;
    write_imported_audio(&output, &source.bytes)?;
    let output_path =
        data_dir_relative_file_path(&state.config.data_dir, &output, "Galgame 音频文件")?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let asset = state
        .media_asset
        .create_media_asset(CreateMediaAssetInput {
            project_id: input.project_id,
            root_project_id,
            source_snapshot_id: Some(source_snapshot_id),
            source_chapter_id: Some(target_context.source_chapter_id),
            source_snapshot_chapter_id: Some(target_context.source_snapshot_chapter_id),
            media_kind: MEDIA_KIND_AUDIO.to_string(),
            origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            file_path: Some(output_path.clone()),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_secs: None,
            fps: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            provenance: CreateMediaProvenanceInput {
                generation_spec_id: None,
                prompt_artifact_id: None,
                provider_job_id: None,
                source_media_asset_id: None,
                source_external_path: Some(source.file_name.clone()),
                imported_from_user: true,
                request_snapshot_json: Some(json!({
                    "source": "galgame_audio_import",
                    "media_kind": "audio",
                    "target_line_id": input.target_line_id,
                    "role": &role,
                    "file_name": source.file_name,
                    "mime_type": source.mime_type,
                    "file_size": source.file_size,
                    "imported_at": Utc::now().to_rfc3339(),
                })),
            },
        })
        .await
        .map_err(|err| {
            let _ = std::fs::remove_file(&output);
            err.to_string()
        })?;
    match bind_galgame_media_inner(
        state,
        BindGalgameMediaInput {
            project_id: input.project_id,
            media_asset_id: asset.id,
            target_kind: GALGAME_ENTITY_LINE.to_string(),
            target_id: input.target_line_id,
            role,
        },
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            cleanup_imported_media_asset(state, asset.id, &output_path).await;
            Err(err)
        }
    }
}

pub async fn export_galgame_webgal(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let detail = state
        .galgame
        .get_script_detail(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    if detail.scenes.is_empty() {
        return Err("Galgame 剧本没有可导出的场".to_string());
    }
    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(detail.script.root_project_id))
        .order_by_asc(character_portrait::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询 Galgame 立绘失败: {err}"))?;
    let bytes = build_webgal_package(&state.config.data_dir, &detail, &portraits)?;
    Ok(ExportFileData {
        file_name: format!(
            "{}-webgal.zip",
            sanitize_export_file_stem(&detail.script.title)
        ),
        mime_type: "application/zip".to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

pub async fn galgame_project_progress(
    state: &AppState,
    project_id: i64,
) -> Result<(Option<galgame_script::Model>, i64, i64), String> {
    match state.galgame.get_script_detail(project_id).await {
        Ok(detail) => {
            let scene_count = detail.scenes.len() as i64;
            let line_count = detail
                .scenes
                .iter()
                .map(|scene| scene.lines.len() as i64)
                .sum::<i64>();
            Ok((Some(detail.script), scene_count, line_count))
        }
        Err(a4n_core::error::AppError::NotFound(_)) => Ok((None, 0, 0)),
        Err(err) => Err(err.to_string()),
    }
}

#[derive(Debug, Clone)]
struct WebgalExportAsset {
    script_name: String,
    zip_path: String,
    data_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WebgalFigureKey {
    profile_id: i64,
    expression: Option<String>,
    pose: Option<String>,
}

#[derive(Default)]
struct WebgalExportAssets {
    scene_backgrounds: BTreeMap<i64, WebgalExportAsset>,
    line_cgs: BTreeMap<i64, WebgalExportAsset>,
    line_bgms: BTreeMap<i64, WebgalExportAsset>,
    line_sfx: BTreeMap<i64, WebgalExportAsset>,
    figures: BTreeMap<WebgalFigureKey, WebgalExportAsset>,
}

fn build_webgal_package(
    data_dir: &Path,
    detail: &GalgameScriptDetail,
    portraits: &[character_portrait::Model],
) -> Result<Vec<u8>, String> {
    if detail.scenes.is_empty() {
        return Err("Galgame 剧本没有可导出的场".to_string());
    }
    validate_webgal_scene_entry_kinds(detail)?;
    let scene_file_by_id = detail
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            (
                scene.scene.id,
                format!("scene_{:03}_{}.txt", index + 1, scene.scene.id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let scene_entry_by_id = detail
        .scenes
        .iter()
        .map(|scene| (scene.scene.id, scene.scene.entry_kind.clone()))
        .collect::<BTreeMap<_, _>>();
    let speaker_name_by_id = detail
        .speaker_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let choices_by_line = detail
        .scenes
        .iter()
        .flat_map(|scene| scene.choices.iter())
        .fold(BTreeMap::<i64, Vec<_>>::new(), |mut acc, choice| {
            acc.entry(choice.line_id).or_default().push(choice);
            acc
        });
    validate_webgal_choice_contract(detail, &scene_file_by_id)?;
    let assets = webgal_export_assets(detail, portraits)?;

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        write_zip_text(
            &mut zip,
            "game/config.txt",
            &webgal_config_text(detail),
            options,
        )?;

        let first_scene = scene_file_by_id
            .get(&detail.scenes[0].scene.id)
            .ok_or_else(|| "Galgame 首场导出文件名缺失".to_string())?;
        write_zip_text(
            &mut zip,
            "game/scene/start.txt",
            &format!("changeScene:{};\n", first_scene),
            options,
        )?;

        for scene in &detail.scenes {
            let file_name = scene_file_by_id
                .get(&scene.scene.id)
                .ok_or_else(|| format!("Galgame 场导出文件名缺失: {}", scene.scene.id))?;
            let script = render_webgal_scene(
                scene,
                &scene_file_by_id,
                &scene_entry_by_id,
                &speaker_name_by_id,
                &choices_by_line,
                &assets,
            )?;
            write_zip_text(
                &mut zip,
                &format!("game/scene/{file_name}"),
                &script,
                options,
            )?;
        }

        let mut written_assets = BTreeSet::new();
        for asset in assets
            .scene_backgrounds
            .values()
            .chain(assets.line_cgs.values())
            .chain(assets.line_bgms.values())
            .chain(assets.line_sfx.values())
            .chain(assets.figures.values())
        {
            if written_assets.insert(asset.zip_path.clone()) {
                write_zip_data_file(data_dir, &mut zip, asset, options)?;
            }
        }

        zip.finish()
            .map_err(|err| format!("WebGAL zip 打包失败: {err}"))?;
    }
    Ok(buffer.into_inner())
}

fn render_webgal_scene(
    scene: &a4n_services::galgame::GalgameSceneDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
    scene_entry_by_id: &BTreeMap<i64, String>,
    speaker_name_by_id: &BTreeMap<i64, String>,
    choices_by_line: &BTreeMap<i64, Vec<&a4n_core::models::galgame::galgame_choice::Model>>,
    assets: &WebgalExportAssets,
) -> Result<String, String> {
    let mut out = String::new();
    let mut current_figure_by_position = BTreeMap::<String, String>::new();
    if let Some(background) = assets.scene_backgrounds.get(&scene.scene.id) {
        out.push_str(&format!("changeBg:{} -next;\n", background.script_name));
    }
    for line in &scene.lines {
        let transition_args = webgal_transition_args(&line.presentation_json, line.id)?;
        if let Some(bgm) = assets.line_bgms.get(&line.id) {
            out.push_str(&format!("bgm:{};\n", bgm.script_name));
        }
        if let Some(sfx) = assets.line_sfx.get(&line.id) {
            out.push_str(&format!("playEffect:{} -next;\n", sfx.script_name));
        }
        if let Some(cg) = assets.line_cgs.get(&line.id) {
            out.push_str(&format!(
                "changeBg:{}{} -next;\n",
                cg.script_name, transition_args
            ));
            out.push_str(&format!(
                "unlockCg:{} -name={};\n",
                cg.script_name,
                webgal_arg(&format!("{} #{} CG", scene.scene.title, line.line_index))
            ));
        }
        if let Some((position, figure)) = webgal_line_figure(line, assets)? {
            let changed = current_figure_by_position
                .get(position)
                .map(|current| current != &figure.script_name)
                .unwrap_or(true);
            if changed {
                out.push_str(&format!(
                    "changeFigure:{} -{} -next;\n",
                    figure.script_name, position
                ));
                current_figure_by_position.insert(position.to_string(), figure.script_name.clone());
            }
        }
        match line.line_type.as_str() {
            LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE => {
                let speaker_profile_id = line.speaker_profile_id.ok_or_else(|| {
                    format!(
                        "Galgame {} 行 {} 缺少 speaker_profile_id，无法导出 WebGAL",
                        line.line_type, line.id
                    )
                })?;
                let speaker = speaker_name_by_id
                    .get(&speaker_profile_id)
                    .map(|name| webgal_speaker(name))
                    .ok_or_else(|| {
                        format!(
                            "Galgame {} 行 {} 绑定的角色档案 {} 不存在，无法导出 WebGAL",
                            line.line_type, line.id, speaker_profile_id
                        )
                    })?;
                out.push_str(&format!("{}:{};\n", speaker, webgal_text(&line.text)));
            }
            LINE_TYPE_NARRATION | LINE_TYPE_STAGE_DIRECTION => {
                out.push_str(&format!("{};\n", webgal_narration(&line.text)));
            }
            LINE_TYPE_CHOICE => {
                let choices = choices_by_line
                    .get(&line.id)
                    .filter(|items| !items.is_empty())
                    .ok_or_else(|| {
                        format!(
                            "Galgame choice 行 {} 缺少 galgame_choice 选项，无法导出 WebGAL",
                            line.id
                        )
                    })?;
                let mut sorted_choices = choices.clone();
                sorted_choices.sort_by_key(|choice| choice.option_index);
                let options = sorted_choices
                    .iter()
                    .map(|choice| {
                        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                            format!(
                                "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                                choice.id
                            )
                        })?;
                        let target = scene_file_by_id.get(&target_scene_id).ok_or_else(|| {
                            format!(
                                "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                                choice.id, target_scene_id
                            )
                        })?;
                        Ok(format!("{}:{}", webgal_arg(&choice.option_text), target))
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .join("|");
                out.push_str(&format!("choose:{};\n", options));
            }
            other => {
                return Err(format!(
                    "Galgame 行 {} line_type 无效，无法导出 WebGAL: {}",
                    line.id, other
                ));
            }
        }
    }
    match scene.scene.next_scene_id {
        Some(next_scene_id) => {
            let next_file = scene_file_by_id.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "Galgame 场 {} next_scene_id 指向不存在的场 {}，无法导出 WebGAL",
                    scene.scene.id, next_scene_id
                )
            })?;
            if scene.scene.entry_kind == SCENE_ENTRY_BRANCH
                && scene_entry_by_id
                    .get(&next_scene_id)
                    .map(|entry_kind| entry_kind.as_str())
                    != Some(SCENE_ENTRY_MAIN)
            {
                return Err(format!(
                    "Galgame branch 场 {} next_scene_id 必须指向主线场，当前指向 {}",
                    scene.scene.id, next_scene_id
                ));
            }
            out.push_str(&format!("changeScene:{};\n", next_file));
        }
        None if scene.scene.entry_kind == SCENE_ENTRY_BRANCH => {
            return Err(format!(
                "Galgame branch 场 {} 缺少 next_scene_id，无法导出 WebGAL",
                scene.scene.id
            ));
        }
        None => {}
    }
    Ok(out)
}

fn validate_webgal_scene_entry_kinds(detail: &GalgameScriptDetail) -> Result<(), String> {
    for scene in &detail.scenes {
        match scene.scene.entry_kind.as_str() {
            SCENE_ENTRY_MAIN | SCENE_ENTRY_BRANCH => {}
            other => {
                return Err(format!(
                    "Galgame 场 {} entry_kind 无效，无法导出 WebGAL: {}",
                    scene.scene.id, other
                ));
            }
        }
    }
    Ok(())
}

fn validate_webgal_choice_contract(
    detail: &GalgameScriptDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
) -> Result<(), String> {
    for scene in &detail.scenes {
        let line_type_by_id = scene
            .lines
            .iter()
            .map(|line| (line.id, line.line_type.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut choices_by_line = BTreeMap::<i64, Vec<&galgame_choice::Model>>::new();
        for choice in &scene.choices {
            match line_type_by_id.get(&choice.line_id).copied() {
                Some(LINE_TYPE_CHOICE) => {}
                Some(line_type) => {
                    return Err(format!(
                        "Galgame choice {} 挂在非 choice 行下，无法导出 WebGAL: line_id={}, line_type={}",
                        choice.id, choice.line_id, line_type
                    ));
                }
                None => {
                    return Err(format!(
                        "Galgame choice {} 引用不存在的行，无法导出 WebGAL: line_id={}",
                        choice.id, choice.line_id
                    ));
                }
            }
            let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                format!(
                    "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                    choice.id
                )
            })?;
            if !scene_file_by_id.contains_key(&target_scene_id) {
                return Err(format!(
                    "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                    choice.id, target_scene_id
                ));
            }
            choices_by_line
                .entry(choice.line_id)
                .or_default()
                .push(choice);
        }

        for line in scene
            .lines
            .iter()
            .filter(|line| line.line_type == LINE_TYPE_CHOICE)
        {
            let choices = choices_by_line
                .get(&line.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if !(2..=3).contains(&choices.len()) {
                return Err(format!(
                    "Galgame choice 行 {} 选项数量必须是 2 到 3 个，无法导出 WebGAL: choice_count={}",
                    line.id,
                    choices.len()
                ));
            }
            let mut option_indexes = BTreeSet::new();
            for choice in choices {
                if choice.option_text.trim().is_empty() {
                    return Err(format!(
                        "Galgame choice {} option_text 不能为空，无法导出 WebGAL",
                        choice.id
                    ));
                }
                if choice.option_index <= 0 {
                    return Err(format!(
                        "Galgame choice {} option_index 必须为正数，无法导出 WebGAL: option_index={}",
                        choice.id, choice.option_index
                    ));
                }
                if !option_indexes.insert(choice.option_index) {
                    return Err(format!(
                        "Galgame choice {} option_index 重复，无法导出 WebGAL: line_id={}, option_index={}",
                        choice.id, choice.line_id, choice.option_index
                    ));
                }
            }
            let expected_indexes = (1..=choices.len() as i32).collect::<BTreeSet<_>>();
            if option_indexes != expected_indexes {
                return Err(format!(
                    "Galgame choice 行 {} option_index 必须从 1 连续编号，无法导出 WebGAL: option_indexes={:?}, expected={:?}",
                    line.id, option_indexes, expected_indexes
                ));
            }
        }
    }
    Ok(())
}

fn webgal_line_figure<'a>(
    line: &galgame_line::Model,
    assets: &'a WebgalExportAssets,
) -> Result<Option<(&'static str, &'a WebgalExportAsset)>, String> {
    let Some(profile_id) = line.speaker_profile_id else {
        return Ok(None);
    };
    let Some(position) = webgal_figure_position(line.position.as_deref(), line.id)? else {
        return Ok(None);
    };
    let key = WebgalFigureKey {
        profile_id,
        expression: line.expression.clone(),
        pose: line.pose.clone(),
    };
    if key.expression.is_some() || key.pose.is_some() {
        return Ok(assets.figures.get(&key).map(|asset| (position, asset)));
    }
    Ok(assets
        .figures
        .get(&WebgalFigureKey {
            profile_id,
            expression: None,
            pose: None,
        })
        .map(|asset| (position, asset)))
}

fn webgal_figure_position(
    value: Option<&str>,
    line_id: i64,
) -> Result<Option<&'static str>, String> {
    match value {
        Some("left") => Ok(Some("left")),
        Some("center") => Ok(Some("center")),
        Some("right") => Ok(Some("right")),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => Err(format!(
            "Galgame 行 {line_id} position 无效，无法导出 WebGAL: {value}"
        )),
        None => Ok(None),
    }
}

fn webgal_transition_args(value: &Value, line_id: i64) -> Result<&'static str, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("Galgame 行 {line_id} presentation_json 必须是 JSON 对象"))?;
    let Some(value) = object.get("transition") else {
        return Ok("");
    };
    let transition = value
        .as_str()
        .ok_or_else(|| format!("Galgame 行 {line_id} transition 必须是字符串"))?
        .trim();
    match transition {
        "" | "fade" | "dissolve" => Ok(""),
        "cut" => Ok(" -duration=0 -enterDuration=0 -exitDuration=0"),
        other => Err(format!("Galgame 行 {line_id} transition 不支持: {other}")),
    }
}

fn webgal_export_assets(
    detail: &GalgameScriptDetail,
    portraits: &[character_portrait::Model],
) -> Result<WebgalExportAssets, String> {
    let mut assets = WebgalExportAssets::default();
    for item in &detail.media_bindings {
        let binding = &item.binding;
        let asset = &item.asset;
        let expected_media_kind =
            galgame_media_binding_required_media_kind(&binding.target_kind, &binding.role)
                .ok_or_else(|| {
                    format!(
                        "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                        binding.id, binding.target_kind, binding.role
                    )
                })?;
        if asset.media_kind != expected_media_kind {
            return Err(format!(
                "Galgame 媒体绑定资产类型不匹配，无法导出 WebGAL: binding_id={}, media_asset_id={}, role={}, expected_media_kind={}, actual_media_kind={}",
                binding.id, asset.id, binding.role, expected_media_kind, asset.media_kind
            ));
        }
        let file_path = asset
            .file_path
            .as_deref()
            .ok_or_else(|| format!("媒体资产 {} 缺少文件路径", asset.id))?;
        let extension = media_file_extension_for_webgal_role(file_path, &binding.role)?;
        match (binding.target_kind.as_str(), binding.role.as_str()) {
            (GALGAME_ENTITY_SCENE, MEDIA_BINDING_ROLE_BACKGROUND) => {
                assets.scene_backgrounds.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "background_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/background/background_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_CG) => {
                assets.line_cgs.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!("cg_{}_{}.{}", binding.target_id, asset.id, extension),
                        zip_path: format!(
                            "game/background/cg_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_BGM) => {
                assets.line_bgms.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "bgm_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/bgm/bgm_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_SFX) => {
                assets.line_sfx.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "sfx_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/vocal/sfx_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            _ => {
                return Err(format!(
                    "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                    binding.id, binding.target_kind, binding.role
                ));
            }
        }
    }
    for portrait in portraits {
        let extension = media_file_extension(&portrait.image_path)?;
        let key = if portrait.is_primary && portrait.expression.is_none() && portrait.pose.is_none()
        {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: None,
                pose: None,
            }
        } else {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: portrait.expression.clone(),
                pose: portrait.pose.clone(),
            }
        };
        assets.figures.insert(
            key,
            WebgalExportAsset {
                script_name: format!(
                    "figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                zip_path: format!(
                    "game/figure/figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                data_path: portrait.image_path.clone(),
            },
        );
    }
    Ok(assets)
}

fn write_zip_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &str,
    content: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|err| format!("写入 WebGAL zip 条目失败: {path}: {err}"))?;
    zip.write_all(content.as_bytes())
        .map_err(|err| format!("写入 WebGAL zip 内容失败: {path}: {err}"))
}

fn write_zip_data_file<W: Write + std::io::Seek>(
    data_dir: &Path,
    zip: &mut zip::ZipWriter<W>,
    asset: &WebgalExportAsset,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    let source = canonicalize_data_file_path(data_dir, &asset.data_path, "WebGAL 媒体文件")?;
    zip.start_file(&asset.zip_path, options)
        .map_err(|err| format!("写入 WebGAL 媒体条目失败: {}: {err}", asset.zip_path))?;
    let bytes = std::fs::read(&source)
        .map_err(|err| format!("读取 WebGAL 媒体文件失败: {}: {err}", source.display()))?;
    zip.write_all(&bytes)
        .map_err(|err| format!("写入 WebGAL 媒体文件失败: {}: {err}", asset.zip_path))
}

fn webgal_config_text(detail: &GalgameScriptDetail) -> String {
    format!(
        "Game_name:{};\nGame_key:a4n_galgame_{};\n",
        webgal_arg(&detail.script.title),
        detail.script.id
    )
}

fn media_file_extension(file_path: &str) -> Result<String, String> {
    let extension = Path::new(file_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {file_path}"))?;
    if extension
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(extension)
    } else {
        Err(format!("媒体文件扩展名非法: {extension}"))
    }
}

fn media_file_extension_for_webgal_role(file_path: &str, role: &str) -> Result<String, String> {
    let extension = media_file_extension(file_path)?;
    match role {
        MEDIA_BINDING_ROLE_BGM | MEDIA_BINDING_ROLE_SFX
            if !matches!(extension.as_str(), "mp3" | "wav" | "ogg") =>
        {
            Err(format!(
                "WebGAL 音频媒体扩展名不支持: role={}, extension={}",
                role, extension
            ))
        }
        _ => Ok(extension),
    }
}

fn sanitize_export_file_stem(value: &str) -> String {
    let stem = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else if ch.is_whitespace() {
                '_'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "galgame".to_string()
    } else {
        stem
    }
}

fn webgal_speaker(value: &str) -> String {
    webgal_arg(value)
}

fn webgal_text(value: &str) -> String {
    value
        .trim()
        .replace(['\r', '\n', '\t'], " ")
        .replace(';', "；")
}

fn webgal_narration(value: &str) -> String {
    webgal_text(value).replace(':', "：")
}

fn webgal_arg(value: &str) -> String {
    webgal_text(value).replace(':', " ").replace('|', "")
}

struct GalgameVisualSpecPlan {
    target_entity_id: i64,
    target_kind: &'static str,
    target_entity_kind: &'static str,
    role: &'static str,
    prompt_kind: &'static str,
    image_type: &'static str,
}

impl GalgameVisualSpecPlan {
    fn from_spec(spec: &generation_spec::Model) -> Result<Self, String> {
        let target_entity_id = spec
            .target_entity_id
            .ok_or_else(|| format!("Galgame 视觉生成意图缺少 target_entity_id: {}", spec.id))?;
        if target_entity_id <= 0 {
            return Err(format!(
                "Galgame 视觉生成意图 target_entity_id 无效: {}",
                spec.id
            ));
        }
        match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
            (GALGAME_TARGET_BACKGROUND, GALGAME_ENTITY_SCENE) => Ok(Self {
                target_entity_id,
                target_kind: GALGAME_TARGET_BACKGROUND,
                target_entity_kind: GALGAME_ENTITY_SCENE,
                role: MEDIA_BINDING_ROLE_BACKGROUND,
                prompt_kind: GALGAME_PROMPT_KIND_BACKGROUND,
                image_type: "galgame_background",
            }),
            (GALGAME_TARGET_CG, GALGAME_ENTITY_LINE) => Ok(Self {
                target_entity_id,
                target_kind: GALGAME_TARGET_CG,
                target_entity_kind: GALGAME_ENTITY_LINE,
                role: MEDIA_BINDING_ROLE_CG,
                prompt_kind: GALGAME_PROMPT_KIND_EVENT_CG,
                image_type: "galgame_cg",
            }),
            _ => Err(format!(
                "Galgame 视觉生成只支持背景/CG spec: spec_id={}, target={}/{}",
                spec.id, spec.target_kind, spec.target_entity_kind
            )),
        }
    }
}

async fn load_galgame_asset_specs(
    state: &AppState,
    input: &RunGalgameAssetGenerationInput,
) -> Result<Vec<generation_spec::Model>, String> {
    let mut query = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(input.project_id))
        .filter(generation_spec::Column::TargetKind.is_in(vec![
            GALGAME_TARGET_BACKGROUND.to_string(),
            GALGAME_TARGET_CG.to_string(),
            GALGAME_TARGET_PORTRAIT_EXPRESSION.to_string(),
        ]))
        .order_by_asc(generation_spec::Column::Id);
    if !input.generation_spec_ids.is_empty() {
        query = query.filter(generation_spec::Column::Id.is_in(input.generation_spec_ids.clone()));
    }
    let mut specs = query
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    if !input.generation_spec_ids.is_empty() && specs.len() != input.generation_spec_ids.len() {
        let found = specs.iter().map(|spec| spec.id).collect::<BTreeSet<_>>();
        let missing = input
            .generation_spec_ids
            .iter()
            .filter(|id| !found.contains(id))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(format!(
            "部分 Galgame 资产生成意图不存在或不属于当前项目: {}",
            missing.join(", ")
        ));
    }
    if let Some(limit) = input.limit {
        specs.truncate(limit);
    }
    Ok(specs)
}

async fn run_galgame_visual_spec_generation(
    state: &AppState,
    user_id: i64,
    input: &RunGalgameAssetGenerationInput,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), String> {
    let plan = GalgameVisualSpecPlan::from_spec(spec)?;
    let target_context = validate_galgame_media_target(
        state,
        input.project_id,
        plan.target_entity_kind,
        plan.target_entity_id,
    )
    .await?;
    validate_galgame_visual_spec_params(spec, &plan, &target_context)?;
    if let Some(item) = completed_galgame_visual_binding(state, spec, &plan).await? {
        return Ok((item, false));
    }

    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_RUNNING).await?;
    let prompt = upsert_galgame_prompt_artifact(state, spec, &plan).await?;
    let image_result = image_generation::generate_galgame_image(
        state,
        user_id,
        GenerateGalgameImageInput {
            project_id: input.project_id,
            prompt: prompt.prompt_text.clone(),
            image_type: plan.image_type.to_string(),
            config_id: input.config_id,
            chapter_id: spec.source_chapter_id,
            generation_spec_id: spec.id,
            target_kind: spec.target_kind.clone(),
            target_entity_kind: spec.target_entity_kind.clone(),
            target_entity_id: plan.target_entity_id,
            workflow_run_id: None,
        },
        None,
    )
    .await;
    let image = match image_result {
        Ok(image) => image,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 图片生成失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    let result = sync_completed_galgame_image_asset(state, spec, &plan, &prompt, &image).await;
    let binding = match result {
        Ok(binding) => binding,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 图片生成后媒体绑定失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED).await?;
    Ok((
        GalgameAssetGenerationItem {
            generation_spec_id: spec.id,
            target_kind: spec.target_kind.clone(),
            target_entity_kind: spec.target_entity_kind.clone(),
            target_entity_id: plan.target_entity_id,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            prompt_artifact_id: Some(prompt.id),
            generated_image_id: Some(image.id),
            portrait_id: None,
            media_asset_id: Some(binding.asset.id),
            binding_id: Some(binding.binding.id),
        },
        true,
    ))
}

async fn completed_galgame_visual_binding(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, String> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let Some(binding) = media_binding::Entity::find()
        .filter(media_binding::Column::TargetKind.eq(plan.target_entity_kind))
        .filter(media_binding::Column::TargetId.eq(plan.target_entity_id))
        .filter(media_binding::Column::Role.eq(plan.role))
        .order_by_desc(media_binding::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    else {
        return Err(format!(
            "Galgame 视觉生成意图已完成但缺少媒体绑定: spec_id={}, target={}/{}",
            spec.id, plan.target_entity_kind, plan.target_entity_id
        ));
    };
    let Some(asset) = media_asset::Entity::find_by_id(binding.media_asset_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    else {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体不存在: spec_id={}, binding_id={}, media_asset_id={}",
            spec.id, binding.id, binding.media_asset_id
        ));
    };
    if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体项目族不一致: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        ));
    }
    if asset.media_kind != MEDIA_KIND_IMAGE {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体类型不是 image: spec_id={}, media_asset_id={}, media_kind={}",
            spec.id, asset.id, asset.media_kind
        ));
    }
    if asset.status != MEDIA_STATUS_COMPLETED {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体未完成: spec_id={}, media_asset_id={}, status={}",
            spec.id, asset.id, asset.status
        ));
    }
    let Some(file_path) = asset.file_path.as_deref() else {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体缺少文件路径: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        ));
    };
    canonicalize_data_file_path(&state.config.data_dir, file_path, "Galgame 视觉媒体文件")
        .map_err(|err| {
            format!(
                "Galgame 视觉生成意图已完成但绑定媒体文件不可用: spec_id={}, media_asset_id={}，{}",
                spec.id, asset.id, err
            )
        })?;
    let provenance = media_provenance::Entity::find_by_id(asset.id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let Some(provenance) = provenance else {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源不存在: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        ));
    };
    if provenance.generation_spec_id != Some(spec.id) {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源未指向该 spec: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        ));
    }
    validate_galgame_visual_media_request_snapshot(
        spec,
        plan,
        asset.id,
        provenance.request_snapshot_json.as_ref(),
    )?;
    Ok(Some(GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: spec.target_kind.clone(),
        target_entity_kind: spec.target_entity_kind.clone(),
        target_entity_id: plan.target_entity_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: provenance.prompt_artifact_id,
        generated_image_id: None,
        portrait_id: None,
        media_asset_id: Some(asset.id),
        binding_id: Some(binding.id),
    }))
}

fn validate_galgame_visual_media_request_snapshot(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    media_asset_id: i64,
    request_snapshot: Option<&Value>,
) -> Result<(), String> {
    let request = request_snapshot
        .ok_or_else(|| {
            format!(
                "Galgame 视觉生成意图已完成但媒体来源缺少 request_snapshot_json: spec_id={}, media_asset_id={}",
                spec.id, media_asset_id
            )
        })?
        .as_object()
        .ok_or_else(|| {
            format!(
                "Galgame 视觉生成意图已完成但媒体来源 request_snapshot_json 必须是对象: spec_id={}, media_asset_id={}",
                spec.id, media_asset_id
            )
        })?;
    let source = request.get("source").and_then(Value::as_str).map(str::trim);
    if source != Some("galgame_asset_generation") {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 source 无效: spec_id={}, media_asset_id={}, source={:?}",
            spec.id, media_asset_id, source
        ));
    }
    validate_galgame_visual_media_request_string(
        request,
        "target_kind",
        spec.target_kind.as_str(),
        spec.id,
        media_asset_id,
    )?;
    validate_galgame_visual_media_request_string(
        request,
        "target_entity_kind",
        spec.target_entity_kind.as_str(),
        spec.id,
        media_asset_id,
    )?;
    validate_galgame_visual_media_request_i64(
        request,
        "target_entity_id",
        plan.target_entity_id,
        spec.id,
        media_asset_id,
    )?;
    Ok(())
}

fn validate_galgame_visual_media_request_string(
    request: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
    spec_id: i64,
    media_asset_id: i64,
) -> Result<(), String> {
    let actual = request
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "Galgame 视觉生成意图已完成但媒体来源 {key} 必须是非空字符串: spec_id={}, media_asset_id={}",
                spec_id, media_asset_id
            )
        })?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 与 spec 目标不一致: spec_id={}, media_asset_id={}, actual={}, expected={}",
            spec_id, media_asset_id, actual, expected
        ));
    }
    Ok(())
}

fn validate_galgame_visual_media_request_i64(
    request: &serde_json::Map<String, Value>,
    key: &str,
    expected: i64,
    spec_id: i64,
    media_asset_id: i64,
) -> Result<(), String> {
    let actual = request.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 必须是整数: spec_id={}, media_asset_id={}",
            spec_id, media_asset_id
        )
    })?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 与 spec 目标不一致: spec_id={}, media_asset_id={}, actual={}, expected={}",
            spec_id, media_asset_id, actual, expected
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct GalgamePortraitSpecPlan {
    profile_id: i64,
    expression: String,
    pose: Option<String>,
    position_samples: Vec<String>,
    source_line_ids: Vec<i64>,
}

impl GalgamePortraitSpecPlan {
    fn from_spec(spec: &generation_spec::Model) -> Result<Self, String> {
        let profile_id = spec
            .target_entity_id
            .ok_or_else(|| format!("Galgame 立绘差分生成意图缺少 target_entity_id: {}", spec.id))?;
        if profile_id <= 0 {
            return Err(format!(
                "Galgame 立绘差分 target_entity_id 无效: {}",
                spec.id
            ));
        }
        if spec.target_kind != GALGAME_TARGET_PORTRAIT_EXPRESSION
            || spec.target_entity_kind != GALGAME_ENTITY_CHARACTER_PROFILE
        {
            return Err(format!(
                "Galgame 立绘差分只支持 portrait_expression/character_profile spec: spec_id={}, target={}/{}",
                spec.id, spec.target_kind, spec.target_entity_kind
            ));
        }
        if let Some(param_profile_id) = spec
            .params_json
            .get("character_profile_id")
            .and_then(Value::as_i64)
            && param_profile_id != profile_id
        {
            return Err(format!(
                "Galgame 立绘差分参数 character_profile_id 与 target_entity_id 不一致: spec_id={}",
                spec.id
            ));
        }
        let expression = json_owned_string(&spec.params_json, "expression");
        let pose = json_owned_string(&spec.params_json, "pose");
        let expression = expression.ok_or_else(|| {
            format!(
                "Galgame 立绘差分生成意图缺少 expression: spec_id={}",
                spec.id
            )
        })?;
        if !GALGAME_EXPRESSIONS.contains(&expression.as_str()) {
            return Err(format!(
                "Galgame 立绘差分生成意图 expression 无效: spec_id={}, expression={}",
                spec.id, expression
            ));
        }
        let variant_key = match pose.as_deref() {
            Some(pose) => format!("expression:{expression}:pose:{pose}"),
            None => format!("expression:{expression}"),
        };
        let expected_key = format!(
            "galgame:{}:portrait:{profile_id}:{variant_key}",
            spec.project_id
        );
        validate_galgame_portrait_canonical_key(spec, &expected_key)?;
        let source_line_ids =
            required_visual_json_i64_array(&spec.params_json, "source_line_ids", spec.id)?;
        let position_samples =
            required_visual_json_string_array(&spec.params_json, "position_samples", spec.id)?;
        Ok(Self {
            profile_id,
            expression,
            pose,
            position_samples,
            source_line_ids,
        })
    }
}

fn validate_galgame_portrait_canonical_key(
    spec: &generation_spec::Model,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key =
        required_visual_json_str(&spec.params_json, "canonical_key", "canonical_key", spec.id)?;
    if canonical_key != expected_key {
        return Err(format!(
            "Galgame 立绘差分生成意图 canonical_key 与目标不一致: spec_id={}, canonical_key={}, expected={}",
            spec.id, canonical_key, expected_key
        ));
    }
    Ok(())
}

async fn run_galgame_portrait_spec_generation(
    state: &AppState,
    user_id: i64,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), String> {
    let plan = GalgamePortraitSpecPlan::from_spec(spec)?;
    let profile = validate_galgame_portrait_spec_target(state, spec, &plan).await?;
    access::verify_project_editor_string(state, user_id, spec.root_project_id).await?;
    if let Some(item) = completed_galgame_portrait(state, spec, &plan).await? {
        return Ok((item, false));
    }

    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_RUNNING).await?;
    let reference_image_ids = match primary_portrait_reference_image_ids(state, &profile).await {
        Ok(ids) => ids,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 立绘差分缺少可用主立绘参考: spec_id={}，{err}",
                spec.id
            ));
        }
    };
    let portrait_result = portrait_generation::generate_portrait(
        state,
        user_id,
        GeneratePortraitInput {
            project_id: spec.root_project_id,
            character_name: profile.name.clone(),
            style: Some(GALGAME_PORTRAIT_VARIANT_STYLE.to_string()),
            expressions: Some(vec![plan.expression.clone()]),
            pose: plan.pose.clone(),
            reference_image_ids,
        },
        None,
        CancellationToken::new(),
    )
    .await;
    let portrait = match portrait_result {
        Ok(portrait) => portrait,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 立绘差分生成失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED).await?;
    Ok((
        galgame_portrait_generation_item(spec, &plan, &portrait),
        true,
    ))
}

async fn validate_galgame_portrait_spec_target(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
) -> Result<protagonist_profile::Model, String> {
    let project = state
        .project
        .get(spec.project_id)
        .await
        .map_err(|err| err.to_string())?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!(
            "Galgame 立绘差分生成意图不属于 Galgame 项目: spec_id={}, project_id={}",
            spec.id, project.id
        ));
    }
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    validate_galgame_portrait_spec_family_context(spec, root_project_id, source_snapshot_id)?;
    validate_galgame_portrait_spec_source_chapter(spec)?;
    let profile = protagonist_profile::Entity::find_by_id(plan.profile_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("Galgame 立绘差分角色档案不存在: {}", plan.profile_id))?;
    if profile.project_id != root_project_id {
        return Err(format!(
            "Galgame 立绘差分角色档案不属于当前 Galgame 根项目: spec_id={}, profile_id={}",
            spec.id, profile.id
        ));
    }
    let snapshot_profile = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(source_snapshot_id))
        .filter(source_snapshot_profile::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq("character"))
        .filter(source_snapshot_profile::Column::SourceProfileId.eq(profile.id))
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    if snapshot_profile.is_none() {
        return Err(format!(
            "Galgame 立绘差分角色档案不属于源快照: spec_id={}, profile_id={}, source_snapshot_id={}",
            spec.id, profile.id, source_snapshot_id
        ));
    }
    validate_galgame_portrait_spec_source_lines(state, spec, plan, &project, source_snapshot_id)
        .await?;
    Ok(profile)
}

fn validate_galgame_portrait_spec_family_context(
    spec: &generation_spec::Model,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), String> {
    if spec.root_project_id != root_project_id
        || spec.source_snapshot_id != Some(source_snapshot_id)
    {
        return Err(format!(
            "Galgame 立绘差分生成意图项目族上下文与项目不一致: spec_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}, project_root_project_id={}, project_source_snapshot_id={}",
            spec.id,
            spec.root_project_id,
            spec.source_snapshot_id,
            root_project_id,
            source_snapshot_id
        ));
    }
    Ok(())
}

fn validate_galgame_portrait_spec_source_chapter(
    spec: &generation_spec::Model,
) -> Result<(), String> {
    match (spec.source_chapter_id, spec.source_snapshot_chapter_id) {
        (None, None) => {}
        _ => {
            return Err(format!(
                "Galgame 立绘差分生成意图必须使用源快照级上下文，source_chapter_id 和 source_snapshot_chapter_id 必须为空: spec_id={}",
                spec.id
            ));
        }
    }
    Ok(())
}

async fn validate_galgame_portrait_spec_source_lines(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
    project: &novel_project::Model,
    source_snapshot_id: i64,
) -> Result<(), String> {
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let mut source_positions = BTreeSet::new();
    for line_id in &plan.source_line_ids {
        let line = galgame_line::Entity::find_by_id(*line_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "Galgame 立绘差分 source_line_ids 引用了不存在的行: spec_id={}, line_id={line_id}",
                    spec.id
                )
            })?;
        if line.speaker_profile_id != Some(plan.profile_id) {
            return Err(format!(
                "Galgame 立绘差分 source_line_ids 行角色与目标不一致: spec_id={}, line_id={}, speaker_profile_id={:?}, expected={}",
                spec.id, line.id, line.speaker_profile_id, plan.profile_id
            ));
        }
        if line.expression.as_deref().map(str::trim) != Some(plan.expression.as_str()) {
            return Err(format!(
                "Galgame 立绘差分 source_line_ids 行 expression 与目标不一致: spec_id={}, line_id={}, expression={:?}, expected={}",
                spec.id, line.id, line.expression, plan.expression
            ));
        }
        if line
            .pose
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            != plan.pose.as_deref()
        {
            return Err(format!(
                "Galgame 立绘差分 source_line_ids 行 pose 与目标不一致: spec_id={}, line_id={}, pose={:?}, expected={:?}",
                spec.id, line.id, line.pose, plan.pose
            ));
        }
        if let Some(position) = normalized_galgame_position(line.position.as_deref()) {
            validate_galgame_position(position, spec.id, line.id)?;
            source_positions.insert(position.to_string());
        }
        let scene = galgame_scene::Entity::find_by_id(line.scene_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "Galgame 立绘差分 source_line_ids 行所属场不存在: spec_id={}, line_id={}, scene_id={}",
                    spec.id, line.id, line.scene_id
                )
            })?;
        let script = galgame_script::Entity::find_by_id(scene.script_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "Galgame 立绘差分 source_line_ids 场所属剧本不存在: spec_id={}, scene_id={}, script_id={}",
                    spec.id, scene.id, scene.script_id
                )
            })?;
        if script.project_id != project.id
            || script.root_project_id != root_project_id
            || script.source_snapshot_id != source_snapshot_id
        {
            return Err(format!(
                "Galgame 立绘差分 source_line_ids 行上下文与项目不一致: spec_id={}, line_id={}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}",
                spec.id,
                line.id,
                script.project_id,
                script.root_project_id,
                script.source_snapshot_id,
                spec.project_id,
                spec.root_project_id,
                spec.source_snapshot_id
            ));
        }
        let snapshot_chapter =
            source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| {
                    format!(
                        "Galgame 立绘差分 source_line_ids 场源快照章节不存在: spec_id={}, scene_id={}, source_snapshot_chapter_id={}",
                        spec.id, scene.id, scene.source_snapshot_chapter_id
                    )
                })?;
        if snapshot_chapter.snapshot_id != source_snapshot_id {
            return Err(format!(
                "Galgame 立绘差分 source_line_ids 场源快照章节与项目不一致: spec_id={}, scene_id={}, source_snapshot_chapter_id={}, expected_source_snapshot_id={}, actual_source_snapshot_id={}",
                spec.id,
                scene.id,
                scene.source_snapshot_chapter_id,
                source_snapshot_id,
                snapshot_chapter.snapshot_id
            ));
        }
    }
    validate_galgame_portrait_position_samples(spec.id, &plan.position_samples, &source_positions)?;
    Ok(())
}

fn normalized_galgame_position(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn validate_galgame_position(position: &str, spec_id: i64, line_id: i64) -> Result<(), String> {
    if matches!(position, "left" | "center" | "right") {
        Ok(())
    } else {
        Err(format!(
            "Galgame 立绘差分 source_line_ids 行 position 无效: spec_id={}, line_id={}, position={position}",
            spec_id, line_id
        ))
    }
}

fn validate_galgame_portrait_position_samples(
    spec_id: i64,
    position_samples: &[String],
    source_positions: &BTreeSet<String>,
) -> Result<(), String> {
    let mut sample_positions = BTreeSet::new();
    for position in position_samples {
        if !matches!(position.as_str(), "left" | "center" | "right") {
            return Err(format!(
                "Galgame 立绘差分 position_samples 包含无效站位: spec_id={}, position={position}",
                spec_id
            ));
        }
        sample_positions.insert(position.clone());
    }
    if sample_positions != *source_positions {
        return Err(format!(
            "Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致: spec_id={}, position_samples={:?}, source_positions={:?}",
            spec_id, sample_positions, source_positions
        ));
    }
    Ok(())
}

async fn completed_galgame_portrait(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, String> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let mut query = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(spec.root_project_id))
        .filter(character_portrait::Column::ProfileId.eq(plan.profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(false));
    query = query.filter(character_portrait::Column::Expression.eq(plan.expression.clone()));
    query = match plan.pose.as_deref() {
        Some(pose) => query.filter(character_portrait::Column::Pose.eq(pose)),
        None => query.filter(character_portrait::Column::Pose.is_null()),
    };
    let Some(portrait) = query
        .order_by_desc(character_portrait::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    else {
        return Err(format!(
            "Galgame 立绘差分生成意图已完成但缺少可用立绘: spec_id={}, profile_id={}, expression={}, pose={:?}",
            spec.id, plan.profile_id, plan.expression, plan.pose
        ));
    };
    canonicalize_data_file_path(
        &state.config.data_dir,
        &portrait.image_path,
        "Galgame 立绘差分",
    )
    .map_err(|err| {
        format!(
            "Galgame 立绘差分生成意图已完成但立绘文件不可用: spec_id={}, portrait_id={}，{}",
            spec.id, portrait.id, err
        )
    })?;
    let summary = portrait_generation::to_summary(&portrait);
    Ok(Some(galgame_portrait_generation_item(spec, plan, &summary)))
}

async fn primary_portrait_reference_image_ids(
    state: &AppState,
    profile: &protagonist_profile::Model,
) -> Result<Vec<i64>, String> {
    let primary = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(profile.project_id))
        .filter(character_portrait::Column::ProfileId.eq(profile.id))
        .filter(character_portrait::Column::IsPrimary.eq(true))
        .order_by_desc(character_portrait::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("角色 {} 没有主立绘", profile.id))?;
    let image = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(profile.project_id))
        .filter(generated_image::Column::FilePath.eq(&primary.image_path))
        .order_by_desc(generated_image::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("主立绘 {} 缺少可用图片镜像记录", primary.id))?;
    Ok(vec![image.id])
}

fn galgame_portrait_generation_item(
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
    portrait: &PortraitSummary,
) -> GalgameAssetGenerationItem {
    GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: spec.target_kind.clone(),
        target_entity_kind: spec.target_entity_kind.clone(),
        target_entity_id: plan.profile_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: None,
        generated_image_id: None,
        portrait_id: Some(portrait.id),
        media_asset_id: None,
        binding_id: None,
    }
}

async fn mark_galgame_generation_spec_status(
    state: &AppState,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), String> {
    generation_spec::ActiveModel {
        id: Set(generation_spec_id),
        status: Set(status.to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

async fn upsert_galgame_prompt_artifact(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<prompt_artifact::Model, String> {
    let existing = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    if let Some(existing) = existing {
        if existing.edited_by_user {
            return Ok(existing);
        }
        let mut active: prompt_artifact::ActiveModel = existing.into();
        active.prompt_kind = Set(plan.prompt_kind.to_string());
        active.prompt_text = Set(render_galgame_image_prompt(
            state.engine.as_ref(),
            spec,
            plan,
        )?);
        active.renderer_version = Set(GALGAME_ASSET_RENDERER_VERSION.to_string());
        active.edited_by_user = Set(false);
        active.updated_at = Set(Utc::now().naive_utc());
        return active
            .update(state.db.as_ref())
            .await
            .map_err(|err| err.to_string());
    }
    let now = Utc::now().naive_utc();
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(spec.id),
        prompt_kind: Set(plan.prompt_kind.to_string()),
        prompt_text: Set(render_galgame_image_prompt(
            state.engine.as_ref(),
            spec,
            plan,
        )?),
        renderer_version: Set(GALGAME_ASSET_RENDERER_VERSION.to_string()),
        edited_by_user: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())
}

fn render_galgame_image_prompt(
    prompt_engine: &dyn PromptEngine,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<String, String> {
    let (template_name, variables) = galgame_image_prompt_variables(spec, plan)?;
    let rendered = prompt_engine
        .render(template_name, &variables, Some(spec.project_id))
        .map_err(|err| err.to_string())?
        .trim()
        .to_string();
    if rendered.is_empty() {
        return Err(format!(
            "Galgame 图像提示词模板渲染为空: spec_id={}, template={template_name}",
            spec.id
        ));
    }
    Ok(rendered)
}

fn galgame_image_prompt_variables(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<PromptVariables, String> {
    match plan.target_kind {
        GALGAME_TARGET_BACKGROUND => galgame_background_prompt_variables(spec),
        GALGAME_TARGET_CG => galgame_event_cg_prompt_variables(spec),
        _ => Err(format!(
            "不支持的 Galgame 视觉 target_kind: {}",
            plan.target_kind
        )),
    }
}

type PromptVariables = (&'static str, HashMap<String, Value>);

fn galgame_background_prompt_variables(
    spec: &generation_spec::Model,
) -> Result<PromptVariables, String> {
    let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
    required_visual_json_str(scene, "title", "scene.title", spec.id)?;

    let mut variables = HashMap::new();
    variables.insert("params".to_string(), spec.params_json.clone());
    variables.insert("scene".to_string(), scene.clone());
    variables.insert(
        "source".to_string(),
        spec.params_json
            .get("source")
            .cloned()
            .unwrap_or(Value::Null),
    );
    variables.insert(
        "canonical_key".to_string(),
        spec.params_json
            .get("canonical_key")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok((GALGAME_BACKGROUND_IMAGE_PROMPT_TEMPLATE, variables))
}

fn galgame_event_cg_prompt_variables(
    spec: &generation_spec::Model,
) -> Result<PromptVariables, String> {
    let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
    required_visual_json_str(scene, "title", "scene.title", spec.id)?;
    let line = required_visual_json_object(&spec.params_json, "line", spec.id)?;
    required_visual_json_str(line, "text", "line.text", spec.id)?;
    let cg_note = required_visual_json_str(&spec.params_json, "cg_note", "cg_note", spec.id)?;
    validate_optional_galgame_cg_importance(&spec.params_json, spec.id)?;

    let mut variables = HashMap::new();
    variables.insert("params".to_string(), spec.params_json.clone());
    variables.insert("scene".to_string(), scene.clone());
    variables.insert("line".to_string(), line.clone());
    variables.insert("cg_note".to_string(), Value::String(cg_note.to_string()));
    variables.insert(
        "cg_importance".to_string(),
        spec.params_json
            .get("cg_importance")
            .cloned()
            .unwrap_or(Value::Null),
    );
    variables.insert(
        "canonical_key".to_string(),
        spec.params_json
            .get("canonical_key")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok((GALGAME_EVENT_CG_PROMPT_TEMPLATE, variables))
}

fn validate_galgame_visual_spec_params(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    target_context: &GalgameTargetContext,
) -> Result<(), String> {
    if spec.project_id != target_context.project_id
        || spec.root_project_id != target_context.root_project_id
        || spec.source_snapshot_id != Some(target_context.source_snapshot_id)
    {
        return Err(format!(
            "Galgame 视觉生成意图项目族上下文与目标不一致: spec_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}",
            spec.id,
            spec.project_id,
            spec.root_project_id,
            spec.source_snapshot_id,
            target_context.project_id,
            target_context.root_project_id,
            target_context.source_snapshot_id
        ));
    }
    if spec.source_chapter_id != Some(target_context.source_chapter_id)
        || spec.source_snapshot_chapter_id != Some(target_context.source_snapshot_chapter_id)
    {
        return Err(format!(
            "Galgame 视觉生成意图源章节上下文与目标不一致: spec_id={}, spec_source_chapter_id={:?}, spec_source_snapshot_chapter_id={:?}, target_source_chapter_id={}, target_source_snapshot_chapter_id={}",
            spec.id,
            spec.source_chapter_id,
            spec.source_snapshot_chapter_id,
            target_context.source_chapter_id,
            target_context.source_snapshot_chapter_id
        ));
    }

    match plan.target_kind {
        GALGAME_TARGET_BACKGROUND => {
            let expected_key = format!(
                "galgame:{}:background:scene:{}",
                spec.project_id, plan.target_entity_id
            );
            validate_galgame_visual_canonical_key(spec, &expected_key)?;
            let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
            validate_visual_json_i64_equals(spec, scene, "id", "scene.id", plan.target_entity_id)?;
            let source = required_visual_json_object(&spec.params_json, "source", spec.id)?;
            validate_visual_json_i64_equals(
                spec,
                source,
                "source_snapshot_chapter_id",
                "source.source_snapshot_chapter_id",
                target_context.source_snapshot_chapter_id,
            )?;
            validate_visual_json_i64_equals(
                spec,
                source,
                "source_chapter_id",
                "source.source_chapter_id",
                target_context.source_chapter_id,
            )?;
        }
        GALGAME_TARGET_CG => {
            let expected_line_id = target_context.line_id.ok_or_else(|| {
                format!(
                    "Galgame CG 生成意图目标上下文缺少行 ID: spec_id={}",
                    spec.id
                )
            })?;
            let expected_key = format!("galgame:{}:cg:line:{expected_line_id}", spec.project_id);
            validate_galgame_visual_canonical_key(spec, &expected_key)?;
            let line = required_visual_json_object(&spec.params_json, "line", spec.id)?;
            validate_visual_json_i64_equals(spec, line, "id", "line.id", expected_line_id)?;
            let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
            validate_visual_json_i64_equals(
                spec,
                scene,
                "id",
                "scene.id",
                target_context.scene_id,
            )?;
            required_visual_json_str(&spec.params_json, "cg_note", "cg_note", spec.id)?;
            validate_optional_galgame_cg_importance(&spec.params_json, spec.id)?;
        }
        other => {
            return Err(format!(
                "不支持的 Galgame 视觉 target_kind: spec_id={}, target_kind={other}",
                spec.id
            ));
        }
    }
    Ok(())
}

fn validate_galgame_visual_canonical_key(
    spec: &generation_spec::Model,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key =
        required_visual_json_str(&spec.params_json, "canonical_key", "canonical_key", spec.id)?;
    if canonical_key != expected_key {
        return Err(format!(
            "Galgame 视觉生成意图 canonical_key 与目标不一致: spec_id={}, canonical_key={}, expected={}",
            spec.id, canonical_key, expected_key
        ));
    }
    Ok(())
}

fn optional_galgame_cg_importance<'a>(
    value: &'a Value,
    label: &str,
) -> Result<Option<&'a str>, String> {
    let Some(raw) = value.get("cg_importance") else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let importance = raw
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{label} 必须是非空字符串或 null"))?;
    if GALGAME_CG_IMPORTANCE.contains(&importance) {
        Ok(Some(importance))
    } else {
        Err(format!("{label} 无效: {importance}"))
    }
}

fn validate_optional_galgame_cg_importance(value: &Value, spec_id: i64) -> Result<(), String> {
    optional_galgame_cg_importance(
        value,
        &format!("Galgame 视觉生成意图参数 cg_importance: spec_id={spec_id}"),
    )
    .map(|_| ())
}

fn json_str<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

fn required_visual_json_object<'a>(
    value: &'a Value,
    field: &str,
    spec_id: i64,
) -> Result<&'a Value, String> {
    match value.get(field) {
        Some(raw) if raw.is_object() => Ok(raw),
        Some(_) => Err(format!(
            "Galgame 视觉生成意图参数 {field} 必须是对象: spec_id={spec_id}"
        )),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"
        )),
    }
}

fn required_visual_json_str<'a>(
    value: &'a Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<&'a str, String> {
    match value.get(field) {
        Some(raw) => raw
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                format!("Galgame 视觉生成意图参数 {path} 必须是非空字符串: spec_id={spec_id}")
            }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}

fn required_visual_json_i64(
    value: &Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    match value.get(field) {
        Some(raw) => raw.as_i64().ok_or_else(|| {
            format!("Galgame 视觉生成意图参数 {path} 必须是整数: spec_id={spec_id}")
        }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}

fn validate_visual_json_i64_equals(
    spec: &generation_spec::Model,
    value: &Value,
    field: &str,
    path: &str,
    expected: i64,
) -> Result<(), String> {
    let actual = required_visual_json_i64(value, field, path, spec.id)?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图参数 {path} 与目标不一致: spec_id={}, actual={}, expected={}",
            spec.id, actual, expected
        ));
    }
    Ok(())
}

fn json_owned_string(value: &Value, field: &str) -> Option<String> {
    json_str(value, field).map(str::to_string)
}

fn required_visual_json_i64_array(
    value: &Value,
    field: &str,
    spec_id: i64,
) -> Result<Vec<i64>, String> {
    let raw = value
        .get(field)
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"))?;
    let items = raw
        .as_array()
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 必须是数组: spec_id={spec_id}"))?;
    if items.is_empty() {
        return Err(format!(
            "Galgame 视觉生成意图参数 {field} 必须是非空数组: spec_id={spec_id}"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = item.as_i64().ok_or_else(|| {
            format!("Galgame 视觉生成意图参数 {field} 必须是整数数组: spec_id={spec_id}")
        })?;
        if id <= 0 {
            return Err(format!(
                "Galgame 视觉生成意图参数 {field} 必须是正整数数组: spec_id={spec_id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "Galgame 视觉生成意图参数 {field} 不能包含重复 ID: spec_id={spec_id}, id={id}"
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn required_visual_json_string_array(
    value: &Value,
    field: &str,
    spec_id: i64,
) -> Result<Vec<String>, String> {
    let raw = value
        .get(field)
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"))?;
    let items = raw
        .as_array()
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 必须是数组: spec_id={spec_id}"))?;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("Galgame 视觉生成意图参数 {field} 必须是非空字符串数组: spec_id={spec_id}")
            })?;
        values.push(value.to_string());
    }
    Ok(values)
}

async fn sync_completed_galgame_image_asset(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    prompt: &prompt_artifact::Model,
    image: &GeneratedImageSummary,
) -> Result<GalgameMediaBindingResult, String> {
    let generated = generated_image::Entity::find_by_id(image.id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("Galgame 图片产物不存在: {}", image.id))?;
    if generated.project_id != spec.project_id {
        return Err(format!("图片不属于当前 Galgame 项目: {}", generated.id));
    }
    let file_path = generated.file_path.trim();
    if file_path.is_empty() {
        return Err(format!(
            "Galgame 图片缺少文件路径，不能登记为媒体资产: {}",
            generated.id
        ));
    }
    let project = state
        .project
        .get(spec.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let asset = state
        .media_asset
        .create_media_asset(CreateMediaAssetInput {
            project_id: spec.project_id,
            root_project_id: spec.root_project_id,
            source_snapshot_id: Some(source_snapshot_id),
            source_chapter_id: spec.source_chapter_id,
            source_snapshot_chapter_id: spec.source_snapshot_chapter_id,
            media_kind: MEDIA_KIND_IMAGE.to_string(),
            origin: MEDIA_ORIGIN_GENERATED.to_string(),
            file_path: Some(file_path.to_string()),
            thumbnail_path: None,
            width: Some(generated.width),
            height: Some(generated.height),
            duration_secs: None,
            fps: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            provenance: CreateMediaProvenanceInput {
                generation_spec_id: Some(spec.id),
                prompt_artifact_id: Some(prompt.id),
                provider_job_id: None,
                source_media_asset_id: None,
                source_external_path: Some(format!("generated_image:{}", generated.id)),
                imported_from_user: false,
                request_snapshot_json: Some(json!({
                    "source": "galgame_asset_generation",
                    "generated_image_id": generated.id,
                    "image_type": generated.image_type,
                    "model": generated.model,
                    "prompt": generated.prompt,
                    "metadata_json": generated.metadata_json,
                    "target_kind": spec.target_kind,
                    "target_entity_kind": spec.target_entity_kind,
                    "target_entity_id": plan.target_entity_id,
                })),
            },
        })
        .await
        .map_err(|err| err.to_string())?;
    match bind_galgame_media_inner(
        state,
        BindGalgameMediaInput {
            project_id: spec.project_id,
            media_asset_id: asset.id,
            target_kind: plan.target_entity_kind.to_string(),
            target_id: plan.target_entity_id,
            role: plan.role.to_string(),
        },
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            cleanup_generated_media_asset_record(state, asset.id).await;
            Err(err)
        }
    }
}

async fn cleanup_generated_media_asset_record(state: &AppState, media_asset_id: i64) {
    let _ = media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.eq(media_asset_id))
        .exec(state.db.as_ref())
        .await;
    let _ = media_asset::Entity::delete_by_id(media_asset_id)
        .exec(state.db.as_ref())
        .await;
}

struct GalgameTargetContext {
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    source_chapter_id: i64,
    source_snapshot_chapter_id: i64,
    scene_id: i64,
    line_id: Option<i64>,
}

struct ImportedAudioSource {
    file_name: String,
    mime_type: &'static str,
    extension: &'static str,
    bytes: Vec<u8>,
    file_size: i64,
}

impl ImportedAudioSource {
    fn from_upload(file_name: &str, mime_type: &str, data_base64: &str) -> Result<Self, String> {
        let file_name = required_text(file_name, "音频文件名")?;
        let (mime_type, extension) = supported_audio_mime(mime_type)?;
        let raw = strip_data_url_prefix(data_base64).trim();
        if raw.is_empty() {
            return Err("音频文件数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("音频文件 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("音频文件不能为空".to_string());
        }
        if bytes.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "音频文件超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        if !file_name_matches_audio_extension(&file_name, extension) {
            return Err(format!(
                "音频文件扩展名与 MIME 不一致，需要 .{} 文件",
                extension
            ));
        }
        let file_size =
            i64::try_from(bytes.len()).map_err(|_| "音频文件过大，无法记录大小".to_string())?;
        Ok(Self {
            file_name,
            mime_type,
            extension,
            bytes,
            file_size,
        })
    }
}

async fn bind_galgame_media_inner(
    state: &AppState,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    if input.media_asset_id <= 0 {
        return Err("media_asset_id 必须是正整数".to_string());
    }
    let target_kind = normalize_galgame_media_target_kind(&input.target_kind)?;
    let role = normalize_galgame_media_role(&target_kind, &input.role)?;
    let target_context =
        validate_galgame_media_target(state, input.project_id, &target_kind, input.target_id)
            .await?;
    let role_rule = validate_galgame_media_role(&target_kind, &role)?;
    let asset = validate_galgame_media_asset(
        state,
        input.project_id,
        input.media_asset_id,
        role_rule.required_media_kind,
        Some(&target_context),
    )
    .await?;
    let tx = state.db.begin().await.map_err(|err| err.to_string())?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(target_kind.clone()))
        .filter(media_binding::Column::TargetId.eq(input.target_id))
        .filter(media_binding::Column::Role.eq(role.clone()))
        .exec(&tx)
        .await
        .map_err(|err| err.to_string())?;
    let binding = media_binding::ActiveModel {
        media_asset_id: Set(asset.id),
        target_kind: Set(target_kind),
        target_id: Set(input.target_id),
        role: Set(role),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(|err| err.to_string())?;
    tx.commit().await.map_err(|err| err.to_string())?;
    Ok(GalgameMediaBindingResult { binding, asset })
}

struct GalgameRoleRule {
    required_media_kind: &'static str,
}

fn normalize_galgame_media_target_kind(value: &str) -> Result<String, String> {
    match value.trim() {
        GALGAME_ENTITY_SCENE => Ok(GALGAME_ENTITY_SCENE.to_string()),
        GALGAME_ENTITY_LINE => Ok(GALGAME_ENTITY_LINE.to_string()),
        other => Err(format!("Galgame 媒体目标类型无效: {other}")),
    }
}

fn normalize_galgame_media_role(target_kind: &str, role: &str) -> Result<String, String> {
    let role = role.trim();
    validate_galgame_media_role(target_kind, role)?;
    Ok(role.to_string())
}

fn validate_galgame_media_role(target_kind: &str, role: &str) -> Result<GalgameRoleRule, String> {
    galgame_media_binding_required_media_kind(target_kind.trim(), role.trim())
        .map(|required_media_kind| GalgameRoleRule {
            required_media_kind,
        })
        .ok_or_else(|| format!("Galgame 媒体绑定目标/角色无效: {} / {}", target_kind, role))
}

fn normalize_audio_role(role: &str) -> Result<String, String> {
    match role.trim() {
        MEDIA_BINDING_ROLE_BGM => Ok(MEDIA_BINDING_ROLE_BGM.to_string()),
        MEDIA_BINDING_ROLE_SFX => Ok(MEDIA_BINDING_ROLE_SFX.to_string()),
        other => Err(format!("Galgame 音频角色无效: {other}")),
    }
}

async fn validate_galgame_media_target(
    state: &AppState,
    project_id: i64,
    target_kind: &str,
    target_id: i64,
) -> Result<GalgameTargetContext, String> {
    if project_id <= 0 || target_id <= 0 {
        return Err("project_id 和 target_id 必须是正整数".to_string());
    }
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|err| err.to_string())?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!("项目不是 Galgame 派生项目: {}", project.id));
    }
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let (scene, line_id) = match target_kind.trim() {
        GALGAME_ENTITY_SCENE => (
            galgame_scene::Entity::find_by_id(target_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("Galgame 场不存在: {target_id}"))?,
            None,
        ),
        GALGAME_ENTITY_LINE => {
            let line = galgame_line::Entity::find_by_id(target_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("Galgame 行不存在: {target_id}"))?;
            let scene = galgame_scene::Entity::find_by_id(line.scene_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("Galgame 场不存在: {}", line.scene_id))?;
            (scene, Some(line.id))
        }
        other => return Err(format!("Galgame 媒体目标类型无效: {other}")),
    };
    let script = galgame_script::Entity::find_by_id(scene.script_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("Galgame 剧本不存在: {}", scene.script_id))?;
    if script.project_id != project.id
        || script.root_project_id != root_project_id
        || script.source_snapshot_id != source_snapshot_id
    {
        return Err("Galgame 媒体目标不属于目标项目族上下文".to_string());
    }
    let snapshot_chapter =
        source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "Galgame 场引用的源快照章节不存在: {}",
                    scene.source_snapshot_chapter_id
                )
            })?;
    if snapshot_chapter.snapshot_id != script.source_snapshot_id {
        return Err("Galgame 场源快照章节与剧本源快照不一致".to_string());
    }
    Ok(GalgameTargetContext {
        project_id: project.id,
        root_project_id,
        source_snapshot_id,
        source_chapter_id: snapshot_chapter.source_chapter_id,
        source_snapshot_chapter_id: snapshot_chapter.id,
        scene_id: scene.id,
        line_id,
    })
}

async fn validate_galgame_media_asset(
    state: &AppState,
    project_id: i64,
    media_asset_id: i64,
    required_media_kind: &str,
    target_context: Option<&GalgameTargetContext>,
) -> Result<media_asset::Model, String> {
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let asset = media_asset::Entity::find_by_id(media_asset_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("媒体资产不存在: {media_asset_id}"))?;
    if asset.project_id != project.id || asset.root_project_id != root_project_id {
        return Err(format!("媒体资产 {} 不属于当前 Galgame 项目族", asset.id));
    }
    if asset.media_kind != required_media_kind {
        return Err(format!(
            "媒体资产 {} 类型不匹配: 需要 {}, 实际 {}",
            asset.id, required_media_kind, asset.media_kind
        ));
    }
    if asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none() {
        return Err(format!("媒体资产 {} 不是可绑定的已完成媒体", asset.id));
    }
    if let Some(context) = target_context {
        validate_galgame_media_asset_source_context(&asset, context)?;
    }
    Ok(asset)
}

fn validate_galgame_media_asset_source_context(
    asset: &media_asset::Model,
    context: &GalgameTargetContext,
) -> Result<(), String> {
    match asset.source_snapshot_id {
        Some(source_snapshot_id) if source_snapshot_id == context.source_snapshot_id => {}
        Some(source_snapshot_id) => {
            return Err(format!(
                "媒体资产 {} 的源快照与目标不一致: asset_source_snapshot_id={}, target_source_snapshot_id={}",
                asset.id, source_snapshot_id, context.source_snapshot_id
            ));
        }
        None => {
            return Err(format!(
                "媒体资产 {} 缺少 source_snapshot_id，不能绑定到 Galgame snapshot-backed 目标",
                asset.id
            ));
        }
    }
    if let Some(source_chapter_id) = asset.source_chapter_id
        && source_chapter_id != context.source_chapter_id
    {
        return Err(format!(
            "媒体资产 {} 的源章节与目标不一致: asset_source_chapter_id={}, target_source_chapter_id={}",
            asset.id, source_chapter_id, context.source_chapter_id
        ));
    }
    if asset.source_chapter_id.is_some() != asset.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "媒体资产 {} 的源章节上下文不完整: source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空",
            asset.id
        ));
    }
    if let Some(snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && snapshot_chapter_id != context.source_snapshot_chapter_id
    {
        return Err(format!(
            "媒体资产 {} 的源快照章节与目标不一致: asset_source_snapshot_chapter_id={}, target_source_snapshot_chapter_id={}",
            asset.id, snapshot_chapter_id, context.source_snapshot_chapter_id
        ));
    }
    Ok(())
}

fn supported_audio_mime(mime_type: &str) -> Result<(&'static str, &'static str), String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Ok(("audio/mpeg", "mp3")),
        "audio/wav" | "audio/x-wav" => Ok(("audio/wav", "wav")),
        "audio/ogg" | "application/ogg" => Ok(("audio/ogg", "ogg")),
        other => Err(format!("Galgame 音频格式不支持: {other}")),
    }
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
}

fn file_name_matches_audio_extension(file_name: &str, extension: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

fn galgame_audio_output_path(
    state: &AppState,
    project_id: i64,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id)
        .root()
        .join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建 Galgame 音频目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("galgame_audio_{stamp}{suffix}.{extension}"));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err("创建 Galgame 音频文件失败：文件名冲突过多".to_string())
}

fn write_imported_audio(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| format!("创建 Galgame 音频文件失败：{err}"))?;
    file.write_all(bytes)
        .map_err(|err| format!("写入 Galgame 音频文件失败：{err}"))
}

async fn cleanup_imported_media_asset(state: &AppState, media_asset_id: i64, file_path: &str) {
    let _ = media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.eq(media_asset_id))
        .exec(state.db.as_ref())
        .await;
    let _ = media_asset::Entity::delete_by_id(media_asset_id)
        .exec(state.db.as_ref())
        .await;
    let _ = std::fs::remove_file(state.config.data_dir.join(file_path));
}

fn required_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::{
        galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
        generation::generation_spec,
    };
    use a4n_engine::prompt::tera_engine::TeraPromptEngine;
    use a4n_services::galgame::{
        GalgameMediaBindingSummary, GalgameSceneDetail, GalgameSpeakerProfile,
        GalgameTextAnchorSummary,
    };
    use chrono::NaiveDateTime;
    use std::io::{Cursor, Read};
    use std::path::Path;
    use tempfile::tempdir;
    use zip::ZipArchive;

    #[test]
    fn galgame_preview_checkpoint_output_rejects_mismatched_parsed_output() {
        let output = valid_scene_segmentation_checkpoint_output();
        let checkpoint_json = json!({
            "output": output,
            "parsed_output": { "scenes": [] }
        });

        let err =
            galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
                .expect_err("mismatched output and parsed_output must not preview");

        assert!(err.contains("output 与 parsed_output 不一致"));
    }

    #[test]
    fn galgame_preview_checkpoint_output_rejects_legacy_parsed_output_only() {
        let checkpoint_json = json!({
            "parsed_output": valid_script_adaptation_checkpoint_output()
        });

        let err = galgame_preview_checkpoint_output(
            GALGAME_CHECKPOINT_LINES_DRAFT,
            Some(&checkpoint_json),
        )
        .expect_err("legacy parsed_output-only checkpoint must not preview");

        assert!(err.contains("缺少 output"));
    }

    #[test]
    fn galgame_preview_checkpoint_output_accepts_matching_output_contract() {
        let output = valid_script_adaptation_checkpoint_output();
        let checkpoint_json = json!({
            "output": output,
            "parsed_output": valid_script_adaptation_checkpoint_output()
        });

        let parsed = galgame_preview_checkpoint_output(
            GALGAME_CHECKPOINT_LINES_DRAFT,
            Some(&checkpoint_json),
        )
        .expect("matching output checkpoint should preview");

        assert_eq!(
            parsed["scene_lines"][0]["lines"][0]["text"],
            json!("你终于来了。")
        );
    }

    #[test]
    fn galgame_preview_checkpoint_output_rejects_missing_output() {
        let checkpoint_json = json!({
            "response_content": "{}"
        });

        let err =
            galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
                .expect_err("completed Galgame checkpoint without output must not preview");

        assert!(err.contains("缺少 output"));
    }

    #[test]
    fn run_galgame_asset_generation_input_rejects_duplicate_spec_ids() {
        let input = RunGalgameAssetGenerationInput {
            project_id: 1,
            generation_spec_ids: vec![10, 10],
            config_id: None,
            limit: None,
        };

        let err = validate_run_galgame_asset_generation_input(&input)
            .expect_err("duplicate generation_spec_ids must fail before querying specs");

        assert!(err.contains("generation_spec_ids 不能包含重复 ID: 10"));
    }

    #[test]
    fn galgame_preview_checkpoint_output_rejects_malformed_output() {
        let checkpoint_json = json!({
            "output": { "scenes": [] }
        });

        let err =
            galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
                .expect_err("malformed Galgame checkpoint output must not preview");

        assert!(err.contains("scenes"));
    }

    #[test]
    fn galgame_preview_checkpoint_output_rejects_unknown_key() {
        let checkpoint_json = json!({
            "output": valid_scene_segmentation_checkpoint_output()
        });

        let err = galgame_preview_checkpoint_output("unknown_stage", Some(&checkpoint_json))
            .expect_err("unknown Galgame checkpoint key must not be accepted");

        assert!(err.contains("checkpoint key 不支持"));
    }

    #[test]
    fn galgame_preview_checkpoint_inputs_require_prior_stage_for_middle_start() {
        let inputs = GalgamePreviewCheckpointInputs {
            required_prior_stage: Some(GalgamePipelineStartStage::CharacterStaging),
            ..Default::default()
        };

        let err = inputs
            .script_adaptation_input()
            .expect_err("start_from_stage=character_staging requires lines_draft checkpoint");

        assert!(err.contains("缺少上游 checkpoint"));
        assert!(err.contains("required_stage=script_adaptation"));
    }

    #[test]
    fn galgame_preview_checkpoint_inputs_do_not_create_placeholder_upstream() {
        let inputs = GalgamePreviewCheckpointInputs::default();

        assert!(inputs.scene_segmentation_input().unwrap().is_none());
        assert!(inputs.script_adaptation_input().unwrap().is_none());
        assert!(inputs.character_staging_input().unwrap().is_none());
    }

    #[test]
    fn galgame_preview_checkpoint_inputs_return_real_completed_upstream() {
        let inputs = GalgamePreviewCheckpointInputs {
            scenes: Some(valid_scene_segmentation_checkpoint_output()),
            lines_draft: Some(valid_script_adaptation_checkpoint_output()),
            ..Default::default()
        };

        let scenes = inputs
            .scene_segmentation_input()
            .expect("scene input")
            .expect("completed scene checkpoint");
        let lines = inputs
            .script_adaptation_input()
            .expect("script input")
            .expect("completed script checkpoint");

        assert_eq!(scenes["scenes"][0]["title"], json!("雨夜重逢"));
        assert_eq!(
            lines["scene_lines"][0]["lines"][0]["text"],
            json!("你终于来了。")
        );
    }

    #[test]
    fn galgame_preview_checkpoint_inputs_require_visual_plan_for_persist_start() {
        let inputs = GalgamePreviewCheckpointInputs {
            scenes: Some(json!({})),
            lines_draft: Some(json!({})),
            lines_staged: Some(json!({})),
            required_prior_stage: Some(GalgamePipelineStartStage::PersistScript),
            ..Default::default()
        };

        let err = inputs
            .validate_required_prior_checkpoints()
            .expect_err("start_from_stage=persist_script requires visual_plan checkpoint");

        assert!(err.contains("缺少上游 checkpoint"));
        assert!(err.contains("required_stage=visual_planning"));
    }

    #[test]
    fn webgal_package_writes_scene_scripts_and_bound_media() {
        let temp = tempdir().expect("temp dir");
        write_fixture_file(temp.path(), "projects/10/images/bg.png", b"bg-bytes");
        write_fixture_file(temp.path(), "projects/10/images/cg.jpg", b"cg-bytes");
        write_fixture_file(temp.path(), "projects/10/audio/bgm.mp3", b"bgm-bytes");
        write_fixture_file(temp.path(), "projects/10/audio/sfx.ogg", b"sfx-bytes");
        let mut detail = webgal_fixture_detail(vec![
            media_summary(
                1,
                11,
                GALGAME_ENTITY_SCENE,
                100,
                MEDIA_BINDING_ROLE_BACKGROUND,
                "projects/10/images/bg.png",
            ),
            media_summary(
                2,
                12,
                GALGAME_ENTITY_LINE,
                1000,
                MEDIA_BINDING_ROLE_CG,
                "projects/10/images/cg.jpg",
            ),
            media_summary(
                3,
                13,
                GALGAME_ENTITY_LINE,
                1000,
                MEDIA_BINDING_ROLE_BGM,
                "projects/10/audio/bgm.mp3",
            ),
            media_summary(
                4,
                14,
                GALGAME_ENTITY_LINE,
                1000,
                MEDIA_BINDING_ROLE_SFX,
                "projects/10/audio/sfx.ogg",
            ),
        ]);
        detail.scenes[0].lines[0].presentation_json = json!({"transition": "cut"});

        let bytes = build_webgal_package(temp.path(), &detail, &[]).expect("webgal package");
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

        assert_zip_text_contains(
            &mut zip,
            "game/config.txt",
            &["Game_name:测试 Galgame;", "Game_key:a4n_galgame_1;"],
        );
        assert_zip_text_contains(
            &mut zip,
            "game/scene/start.txt",
            &["changeScene:scene_001_100.txt;"],
        );
        assert_zip_text_contains(
            &mut zip,
            "game/scene/scene_001_100.txt",
            &[
                "changeBg:background_100_11.png -next;",
                "bgm:bgm_1000_13.mp3;",
                "playEffect:sfx_1000_14.ogg -next;",
                "changeBg:cg_1000_12.jpg -duration=0 -enterDuration=0 -exitDuration=0 -next;",
                "unlockCg:cg_1000_12.jpg -name=第一场 #1 CG;",
                "林晓雨:你好；世界 下一行;",
                "雨停了;",
                "choose:继续 回想:scene_002_200.txt|留在原地:scene_002_200.txt;",
                "changeScene:scene_002_200.txt;",
            ],
        );
        assert_zip_text_contains(&mut zip, "game/scene/scene_002_200.txt", &["第二场开始;"]);
        assert_zip_bytes(
            &mut zip,
            "game/background/background_100_11.png",
            b"bg-bytes",
        );
        assert_zip_bytes(&mut zip, "game/background/cg_1000_12.jpg", b"cg-bytes");
        assert_zip_bytes(&mut zip, "game/bgm/bgm_1000_13.mp3", b"bgm-bytes");
        assert_zip_bytes(&mut zip, "game/vocal/sfx_1000_14.ogg", b"sfx-bytes");
    }

    #[test]
    fn webgal_package_writes_matching_character_figures() {
        let temp = tempdir().expect("temp dir");
        write_fixture_file(
            temp.path(),
            "projects/1/portraits/smile.png",
            b"figure-bytes",
        );
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].expression = Some("smile".to_string());
        detail.scenes[0].lines[0].pose = Some("standing".to_string());
        detail.scenes[0].lines[0].position = Some("left".to_string());
        let portraits = vec![portrait_model(
            50,
            1,
            Some("smile"),
            Some("standing"),
            false,
            "projects/1/portraits/smile.png",
        )];

        let bytes = build_webgal_package(temp.path(), &detail, &portraits).expect("webgal package");
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

        assert_zip_text_contains(
            &mut zip,
            "game/scene/scene_001_100.txt",
            &[
                "changeFigure:figure_1_50.png -left -next;",
                "林晓雨:你好；世界 下一行;",
            ],
        );
        assert_zip_bytes(&mut zip, "game/figure/figure_1_50.png", b"figure-bytes");
    }

    #[test]
    fn webgal_package_rejects_missing_bound_media_file() {
        let temp = tempdir().expect("temp dir");
        let detail = webgal_fixture_detail(vec![media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/missing.png",
        )]);

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("missing media file should fail");

        assert!(err.contains("WebGAL 媒体文件") || err.contains("无法读取"));
    }

    #[test]
    fn webgal_package_rejects_invalid_media_binding_pair() {
        let temp = tempdir().expect("temp dir");
        write_fixture_file(temp.path(), "projects/10/images/bg.png", b"bg-bytes");
        let detail = webgal_fixture_detail(vec![media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_CG,
            "projects/10/images/bg.png",
        )]);

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("invalid Galgame media binding pair must fail");

        assert!(err.contains("媒体绑定目标/角色无效"));
        assert!(err.contains("target_kind=galgame_scene"));
        assert!(err.contains("role=cg"));
    }

    #[test]
    fn galgame_media_asset_source_context_accepts_same_snapshot_without_chapter() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_chapter_id = None;
        asset.source_snapshot_chapter_id = None;

        validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect("same source snapshot can be bound without a chapter-specific asset context");
    }

    #[test]
    fn galgame_media_asset_source_context_rejects_missing_snapshot() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_snapshot_id = None;

        let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect_err("Galgame media bindings must keep source snapshot context");

        assert!(err.contains("缺少 source_snapshot_id"));
    }

    #[test]
    fn galgame_media_asset_source_context_rejects_snapshot_mismatch() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_snapshot_id = Some(21);

        let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect_err("Galgame media source snapshot must match the target script snapshot");

        assert!(err.contains("源快照与目标不一致"));
        assert!(err.contains("asset_source_snapshot_id=21"));
        assert!(err.contains("target_source_snapshot_id=20"));
    }

    #[test]
    fn galgame_media_asset_source_context_rejects_snapshot_chapter_mismatch() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_snapshot_chapter_id = Some(41);

        let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect_err("chapter-scoped Galgame media must match the target scene chapter");

        assert!(err.contains("源快照章节与目标不一致"));
        assert!(err.contains("asset_source_snapshot_chapter_id=41"));
        assert!(err.contains("target_source_snapshot_chapter_id=40"));
    }

    #[test]
    fn galgame_media_asset_source_context_rejects_partial_chapter_context() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_snapshot_chapter_id = None;

        let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect_err("source_chapter_id without source_snapshot_chapter_id must fail");

        assert!(err.contains("源章节上下文不完整"));
        assert!(err.contains("source_snapshot_chapter_id"));
    }

    #[test]
    fn galgame_media_asset_source_context_rejects_snapshot_chapter_without_source_chapter() {
        let mut asset = media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        )
        .asset;
        asset.source_chapter_id = None;

        let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
            .expect_err("source_snapshot_chapter_id without source_chapter_id must fail");

        assert!(err.contains("源章节上下文不完整"));
        assert!(err.contains("source_chapter_id"));
    }

    #[test]
    fn webgal_package_rejects_media_binding_kind_mismatch() {
        let temp = tempdir().expect("temp dir");
        write_fixture_file(
            temp.path(),
            "projects/10/images/not-audio.png",
            b"image-bytes",
        );
        let detail = webgal_fixture_detail(vec![media_summary_with_kind(
            1,
            11,
            GALGAME_ENTITY_LINE,
            1000,
            MEDIA_BINDING_ROLE_BGM,
            MEDIA_KIND_IMAGE,
            "projects/10/images/not-audio.png",
        )]);

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("media kind mismatch must fail");

        assert!(err.contains("媒体绑定资产类型不匹配"));
        assert!(err.contains("expected_media_kind=audio"));
        assert!(err.contains("actual_media_kind=image"));
    }

    #[test]
    fn webgal_package_rejects_unsupported_audio_extension() {
        let temp = tempdir().expect("temp dir");
        write_fixture_file(temp.path(), "projects/10/audio/bgm.txt", b"audio-bytes");
        let detail = webgal_fixture_detail(vec![media_summary_with_kind(
            1,
            11,
            GALGAME_ENTITY_LINE,
            1000,
            MEDIA_BINDING_ROLE_BGM,
            MEDIA_KIND_AUDIO,
            "projects/10/audio/bgm.txt",
        )]);

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("unsupported WebGAL audio extension must fail");

        assert!(err.contains("WebGAL 音频媒体扩展名不支持"));
        assert!(err.contains("role=bgm"));
        assert!(err.contains("extension=txt"));
    }

    #[test]
    fn webgal_package_rejects_unknown_transition() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].presentation_json = json!({"transition": "spin"});

        let err =
            build_webgal_package(temp.path(), &detail, &[]).expect_err("unknown transition fails");

        assert!(err.contains("transition 不支持: spin"));
    }

    #[test]
    fn webgal_package_rejects_invalid_figure_position() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].position = Some("foreground".to_string());

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("invalid figure position must fail instead of skipping changeFigure");

        assert!(err.contains("position 无效"));
        assert!(err.contains("foreground"));
    }

    #[test]
    fn webgal_package_rejects_dialogue_without_speaker_profile() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].speaker_profile_id = None;

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("dialogue without speaker should fail");

        assert!(err.contains("缺少 speaker_profile_id"));
    }

    #[test]
    fn webgal_package_rejects_dialogue_with_missing_speaker_profile() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].speaker_profile_id = Some(999);

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("dialogue with missing speaker profile should fail");

        assert!(err.contains("绑定的角色档案 999 不存在"));
    }

    #[test]
    fn webgal_package_rejects_choice_line_without_choices() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].choices.clear();

        let err =
            build_webgal_package(temp.path(), &detail, &[]).expect_err("choice without options");

        assert!(err.contains("选项数量必须是 2 到 3 个"));
        assert!(err.contains("choice_count=0"));
    }

    #[test]
    fn webgal_package_rejects_choice_line_with_single_option() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].choices.truncate(1);

        let err =
            build_webgal_package(temp.path(), &detail, &[]).expect_err("single choice option");

        assert!(err.contains("选项数量必须是 2 到 3 个"));
        assert!(err.contains("choice_count=1"));
    }

    #[test]
    fn webgal_package_rejects_choice_line_with_empty_option_text() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].choices[0].option_text = "  ".to_string();

        let err =
            build_webgal_package(temp.path(), &detail, &[]).expect_err("empty choice option text");

        assert!(err.contains("option_text 不能为空"));
        assert!(err.contains("choice 1"));
    }

    #[test]
    fn webgal_package_rejects_choice_line_with_non_contiguous_option_index() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].choices[1].option_index = 3;

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("non-contiguous choice option indexes");

        assert!(err.contains("option_index 必须从 1 连续编号"));
        assert!(err.contains("expected"));
    }

    #[test]
    fn webgal_package_rejects_choice_attached_to_non_choice_line() {
        let temp = tempdir().expect("temp dir");
        let now = fixed_time();
        let mut detail = webgal_fixture_detail(Vec::new());
        detail
            .scenes
            .get_mut(0)
            .expect("fixture first scene")
            .choices
            .push(choice_model(3, 1001, 3, "错误残留", 200, now));

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("choice attached to a non-choice line");

        assert!(err.contains("挂在非 choice 行下"));
        assert!(err.contains("line_id=1001"));
    }

    #[test]
    fn webgal_package_rejects_empty_script() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes.clear();

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("empty Galgame script cannot be exported");

        assert!(err.contains("没有可导出的场"));
    }

    #[test]
    fn webgal_package_rejects_unknown_line_type() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].lines[0].line_type = "jump".to_string();

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("unknown Galgame line_type cannot be exported as narration");

        assert!(err.contains("line_type 无效"));
        assert!(err.contains("jump"));
    }

    #[test]
    fn webgal_package_rejects_unknown_scene_entry_kind() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].scene.entry_kind = "side_path".to_string();

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("unknown Galgame scene entry_kind cannot be exported as main");

        assert!(err.contains("entry_kind 无效"));
        assert!(err.contains("side_path"));
    }

    #[test]
    fn webgal_package_rejects_missing_next_scene_target() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[0].scene.next_scene_id = Some(999);

        let err =
            build_webgal_package(temp.path(), &detail, &[]).expect_err("missing next scene target");

        assert!(err.contains("next_scene_id 指向不存在的场 999"));
    }

    #[test]
    fn webgal_package_rejects_branch_scene_without_return_target() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[1].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();
        detail.scenes[1].scene.next_scene_id = None;

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("branch scene without return target");

        assert!(err.contains("branch 场 200 缺少 next_scene_id"));
    }

    #[test]
    fn webgal_package_rejects_branch_scene_returning_to_branch() {
        let temp = tempdir().expect("temp dir");
        let mut detail = webgal_fixture_detail(Vec::new());
        detail.scenes[1].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();
        detail.scenes[1].scene.next_scene_id = Some(300);
        detail.scenes.push(GalgameSceneDetail {
            scene: scene_model(300, 3, "另一支线", None, fixed_time()),
            lines: vec![line_model(
                3000,
                300,
                1,
                "narration",
                None,
                "另一条支线",
                fixed_time(),
            )],
            choices: Vec::<galgame_choice::Model>::new(),
        });
        detail.scenes[2].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();

        let err = build_webgal_package(temp.path(), &detail, &[])
            .expect_err("branch scene returning to branch");

        assert!(err.contains("next_scene_id 必须指向主线场"));
    }

    #[test]
    fn galgame_background_prompt_rejects_missing_required_scene_title() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "scene": {
                    "location": "车站",
                    "time_of_day": "夜晚",
                    "background_note": "雨夜月台"
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

        let err = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
            .expect_err("background prompt should reject missing title");

        assert!(err.contains("scene.title 缺失"));
    }

    #[test]
    fn galgame_background_prompt_renders_registered_template() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": {
                    "id": 100,
                    "title": "雨夜车站",
                    "location": "旧车站月台",
                    "time_of_day": "夜晚",
                    "background_note": "雨后的冷色灯光和空旷月台"
                },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
        let prompt = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
            .expect("background prompt should render");

        assert!(prompt.contains("视觉小说 Galgame 背景图"));
        assert!(prompt.contains("场景标题：雨夜车站"));
        assert!(prompt.contains("地点：旧车站月台"));
        assert!(prompt.contains("背景规划：雨后的冷色灯光和空旷月台"));
        assert!(!prompt.contains("Scene title:"));
        assert!(!prompt.contains("no watermark"));
    }

    #[test]
    fn galgame_event_cg_prompt_rejects_missing_cg_note() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "scene": {
                    "title": "雨夜车站",
                    "location": "车站"
                },
                "line": {
                    "text": "她回过头。"
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

        let err = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
            .expect_err("event CG prompt should reject missing cg_note");

        assert!(err.contains("cg_note 缺失"));
    }

    #[test]
    fn galgame_event_cg_prompt_omits_optional_null_fields_without_defaults() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "scene": {
                    "title": "雨夜车站",
                    "location": null,
                    "time_of_day": null
                },
                "line": {
                    "text": "她回过头。",
                    "expression": null,
                    "pose": null
                },
                "cg_note": "两人在雨中对视",
                "cg_importance": "high"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let prompt = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
            .expect("event CG prompt should allow explicit null optional fields");

        assert!(prompt.contains("场景标题：雨夜车站"));
        assert!(prompt.contains("事件文本：她回过头。"));
        assert!(prompt.contains("CG 方向：两人在雨中对视"));
        assert!(prompt.contains("重要程度：high"));
        assert!(!prompt.contains("neutral"));
        assert!(!prompt.contains("natural"));
        assert!(!prompt.contains("未指定"));
    }

    #[test]
    fn visual_spec_params_accept_background_contract() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
        let context = background_target_context();

        validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect("background params should match target context");
    }

    #[test]
    fn visual_spec_params_accept_cg_contract() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": {
                    "id": 1000,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "cg_note": "两人在雨中对视",
                "cg_importance": "critical"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect("cg params should match target context");
    }

    #[test]
    fn visual_spec_params_rejects_background_scene_id_mismatch() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": {
                    "id": 200,
                    "title": "雨夜车站"
                },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
        let context = background_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("scene.id mismatch should fail");

        assert!(err.contains("scene.id 与目标不一致"));
        assert!(err.contains("actual=200"));
        assert!(err.contains("expected=100"));
    }

    #[test]
    fn visual_spec_params_rejects_cg_line_id_mismatch() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": {
                    "id": 999,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "cg_note": "两人在雨中对视"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("line.id mismatch should fail");

        assert!(err.contains("line.id 与目标不一致"));
        assert!(err.contains("actual=999"));
        assert!(err.contains("expected=1000"));
    }

    #[test]
    fn visual_spec_params_rejects_cg_scene_id_mismatch() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": {
                    "id": 1000,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 200,
                    "title": "另一场"
                },
                "cg_note": "两人在雨中对视"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("scene.id mismatch should fail");

        assert!(err.contains("scene.id 与目标不一致"));
        assert!(err.contains("actual=200"));
        assert!(err.contains("expected=100"));
    }

    #[test]
    fn visual_spec_params_rejects_cg_missing_note() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": {
                    "id": 1000,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("missing cg_note should fail");

        assert!(err.contains("cg_note 缺失"));
    }

    #[test]
    fn visual_spec_params_rejects_invalid_cg_importance() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": {
                    "id": 1000,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "cg_note": "两人在雨中对视",
                "cg_importance": "major"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("invalid cg_importance should fail");

        assert!(err.contains("cg_importance"));
        assert!(err.contains("major"));
    }

    #[test]
    fn visual_spec_params_rejects_source_context_mismatch() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(41),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
        let context = background_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("spec source context mismatch should fail");

        assert!(err.contains("源章节上下文与目标不一致"));
        assert!(err.contains("spec_source_snapshot_chapter_id=Some(41)"));
    }

    #[test]
    fn visual_spec_params_rejects_background_source_params_mismatch() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "source": {
                    "source_snapshot_chapter_id": 41,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
        let context = background_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("params source context mismatch should fail");

        assert!(err.contains("source.source_snapshot_chapter_id 与目标不一致"));
        assert!(err.contains("actual=41"));
        assert!(err.contains("expected=40"));
    }

    #[test]
    fn visual_spec_params_rejects_canonical_key_mismatch() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:999",
                "line": {
                    "id": 1000,
                    "text": "她回过头。"
                },
                "scene": {
                    "id": 100,
                    "title": "雨夜车站"
                },
                "cg_note": "两人在雨中对视"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
        let context = cg_target_context();

        let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
            .expect_err("canonical key mismatch should fail");

        assert!(err.contains("canonical_key 与目标不一致"));
        assert!(err.contains("galgame:10:cg:line:999"));
        assert!(err.contains("galgame:10:cg:line:1000"));
    }

    #[test]
    fn completed_visual_media_request_accepts_galgame_asset_generation_contract() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": { "id": 100, "title": "雨夜车站" },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

        validate_galgame_visual_media_request_snapshot(
            &spec,
            &plan,
            701,
            Some(&json!({
                "source": "galgame_asset_generation",
                "target_kind": "galgame_background",
                "target_entity_kind": "galgame_scene",
                "target_entity_id": 100
            })),
        )
        .expect("valid completed visual media provenance should pass");
    }

    #[test]
    fn completed_visual_media_request_rejects_missing_snapshot() {
        let spec = background_visual_spec(
            10,
            Some(100),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:background:scene:100",
                "scene": { "id": 100, "title": "雨夜车站" },
                "source": {
                    "source_snapshot_chapter_id": 40,
                    "source_chapter_id": 30
                }
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

        let err = validate_galgame_visual_media_request_snapshot(&spec, &plan, 701, None)
            .expect_err("completed visual media provenance must keep request snapshot");

        assert!(err.contains("缺少 request_snapshot_json"));
        assert!(err.contains("media_asset_id=701"));
    }

    #[test]
    fn completed_visual_media_request_rejects_wrong_source() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": { "id": 1000, "text": "她回过头。" },
                "scene": { "id": 100, "title": "雨夜车站" },
                "cg_note": "两人在雨中对视"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

        let err = validate_galgame_visual_media_request_snapshot(
            &spec,
            &plan,
            702,
            Some(&json!({
                "source": "manual_binding",
                "target_kind": "galgame_cg",
                "target_entity_kind": "galgame_line",
                "target_entity_id": 1000
            })),
        )
        .expect_err("completed visual media provenance source must be Galgame asset generation");

        assert!(err.contains("媒体来源 source 无效"));
        assert!(err.contains("manual_binding"));
    }

    #[test]
    fn completed_visual_media_request_rejects_target_mismatch() {
        let spec = cg_visual_spec(
            10,
            Some(1000),
            Some(30),
            Some(40),
            json!({
                "canonical_key": "galgame:10:cg:line:1000",
                "line": { "id": 1000, "text": "她回过头。" },
                "scene": { "id": 100, "title": "雨夜车站" },
                "cg_note": "两人在雨中对视"
            }),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

        let err = validate_galgame_visual_media_request_snapshot(
            &spec,
            &plan,
            702,
            Some(&json!({
                "source": "galgame_asset_generation",
                "target_kind": "galgame_cg",
                "target_entity_kind": "galgame_line",
                "target_entity_id": 999
            })),
        )
        .expect_err("completed visual media provenance target must match the spec");

        assert!(err.contains("target_entity_id 与 spec 目标不一致"));
        assert!(err.contains("actual=999"));
        assert!(err.contains("expected=1000"));
    }

    #[test]
    fn portrait_spec_plan_accepts_expression_and_pose() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile:pose:standing",
                "character_profile_id": 42,
                "expression": "smile",
                "pose": "standing",
                "position_samples": ["left", "right"],
                "source_line_ids": [1000, 1001]
            }),
        );

        let plan = GalgamePortraitSpecPlan::from_spec(&spec).expect("portrait spec plan");

        assert_eq!(plan.profile_id, 42);
        assert_eq!(plan.expression, "smile");
        assert_eq!(plan.pose.as_deref(), Some("standing"));
        assert_eq!(plan.position_samples, vec!["left", "right"]);
        assert_eq!(plan.source_line_ids, vec![1000, 1001]);
    }

    #[test]
    fn portrait_spec_plan_rejects_missing_expression() {
        let spec = portrait_expression_spec(10, Some(42), json!({"character_profile_id": 42}));

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing variant fields");

        assert!(err.contains("缺少 expression"));
    }

    #[test]
    fn portrait_spec_plan_rejects_invalid_expression() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "character_profile_id": 42,
                "expression": "ecstatic"
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid expression");

        assert!(err.contains("expression 无效"));
        assert!(err.contains("ecstatic"));
    }

    #[test]
    fn portrait_spec_plan_rejects_pose_without_expression() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "character_profile_id": 42,
                "pose": "standing"
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing expression");

        assert!(err.contains("缺少 expression"));
    }

    #[test]
    fn portrait_spec_plan_rejects_profile_id_mismatch() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "character_profile_id": 99,
                "expression": "smile"
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("profile mismatch");

        assert!(err.contains("character_profile_id 与 target_entity_id 不一致"));
    }

    #[test]
    fn portrait_spec_plan_rejects_non_portrait_target() {
        let mut spec = portrait_expression_spec(10, Some(42), json!({"expression": "smile"}));
        spec.target_kind = GALGAME_TARGET_BACKGROUND.to_string();
        spec.target_entity_kind = GALGAME_ENTITY_SCENE.to_string();

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid target");

        assert!(err.contains("只支持 portrait_expression/character_profile spec"));
    }

    #[test]
    fn portrait_spec_plan_rejects_canonical_key_mismatch() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:99:expression:smile",
                "character_profile_id": 42,
                "expression": "smile"
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("canonical key mismatch");

        assert!(err.contains("canonical_key 与目标不一致"));
        assert!(err.contains("galgame:10:portrait:99:expression:smile"));
        assert!(err.contains("galgame:10:portrait:42:expression:smile"));
    }

    #[test]
    fn portrait_spec_plan_rejects_missing_source_line_ids() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile",
                "character_profile_id": 42,
                "expression": "smile",
                "position_samples": []
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing source lines");

        assert!(err.contains("source_line_ids 缺失"));
    }

    #[test]
    fn portrait_spec_plan_rejects_empty_source_line_ids() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile",
                "character_profile_id": 42,
                "expression": "smile",
                "position_samples": [],
                "source_line_ids": []
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("empty source lines");

        assert!(err.contains("source_line_ids 必须是非空数组"));
    }

    #[test]
    fn portrait_spec_plan_rejects_invalid_position_samples() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile",
                "character_profile_id": 42,
                "expression": "smile",
                "position_samples": ["left", ""],
                "source_line_ids": [1000]
            }),
        );

        let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid position sample");

        assert!(err.contains("position_samples 必须是非空字符串数组"));
    }

    #[test]
    fn portrait_position_samples_must_match_source_positions() {
        let samples = vec!["left".to_string()];
        let source_positions = BTreeSet::from(["right".to_string()]);

        let err = validate_galgame_portrait_position_samples(1, &samples, &source_positions)
            .expect_err("portrait position samples must match source line positions");

        assert!(err.contains("position_samples 与 source_line_ids 行站位不一致"));
    }

    #[test]
    fn portrait_position_samples_reject_invalid_position() {
        let samples = vec!["foreground".to_string()];
        let source_positions = BTreeSet::new();

        let err = validate_galgame_portrait_position_samples(1, &samples, &source_positions)
            .expect_err("portrait position samples must use controlled positions");

        assert!(err.contains("position_samples 包含无效站位"));
        assert!(err.contains("foreground"));
    }

    #[test]
    fn portrait_spec_family_context_accepts_matching_project_context() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile",
                "character_profile_id": 42,
                "expression": "smile"
            }),
        );

        validate_galgame_portrait_spec_family_context(&spec, 1, 20)
            .expect("matching family context should pass");
    }

    #[test]
    fn portrait_spec_family_context_rejects_source_snapshot_mismatch() {
        let spec = portrait_expression_spec(
            10,
            Some(42),
            json!({
                "canonical_key": "galgame:10:portrait:42:expression:smile",
                "character_profile_id": 42,
                "expression": "smile"
            }),
        );

        let err = validate_galgame_portrait_spec_family_context(&spec, 1, 21)
            .expect_err("source snapshot mismatch should fail");

        assert!(err.contains("项目族上下文与项目不一致"));
        assert!(err.contains("spec_source_snapshot_id=Some(20)"));
        assert!(err.contains("project_source_snapshot_id=21"));
    }

    fn webgal_fixture_detail(
        media_bindings: Vec<GalgameMediaBindingSummary>,
    ) -> GalgameScriptDetail {
        let now = fixed_time();
        GalgameScriptDetail {
            script: galgame_script::Model {
                id: 1,
                project_id: 10,
                root_project_id: 1,
                source_snapshot_id: 20,
                title: "测试 Galgame".to_string(),
                style_config_json: json!({}),
                status: "ready".to_string(),
                created_at: now,
                updated_at: now,
            },
            speaker_profiles: vec![GalgameSpeakerProfile {
                id: 1,
                name: "林晓雨".to_string(),
            }],
            text_anchors: Vec::<GalgameTextAnchorSummary>::new(),
            media_bindings,
            scenes: vec![
                GalgameSceneDetail {
                    scene: scene_model(100, 1, "第一场", Some(200), now),
                    lines: vec![
                        line_model(1000, 100, 1, "dialogue", Some(1), "你好;世界\n下一行", now),
                        line_model(1001, 100, 2, "narration", None, "雨停了", now),
                        line_model(1002, 100, 3, "choice", None, "选择", now),
                    ],
                    choices: vec![
                        choice_model(1, 1002, 1, "继续:回想", 200, now),
                        choice_model(2, 1002, 2, "留在|原地", 200, now),
                    ],
                },
                GalgameSceneDetail {
                    scene: scene_model(200, 2, "第二场", None, now),
                    lines: vec![line_model(
                        2000,
                        200,
                        1,
                        "narration",
                        None,
                        "第二场开始",
                        now,
                    )],
                    choices: Vec::<galgame_choice::Model>::new(),
                },
            ],
        }
    }

    fn scene_model(
        id: i64,
        scene_index: i32,
        title: &str,
        next_scene_id: Option<i64>,
        now: NaiveDateTime,
    ) -> galgame_scene::Model {
        galgame_scene::Model {
            id,
            script_id: 1,
            source_snapshot_chapter_id: id + 1000,
            scene_index,
            title: title.to_string(),
            location: None,
            time_of_day: None,
            participants_json: json!([]),
            background_note: None,
            entry_kind: "main".to_string(),
            next_scene_id,
            created_at: now,
            updated_at: now,
        }
    }

    fn line_model(
        id: i64,
        scene_id: i64,
        line_index: i32,
        line_type: &str,
        speaker_profile_id: Option<i64>,
        text: &str,
        now: NaiveDateTime,
    ) -> galgame_line::Model {
        galgame_line::Model {
            id,
            scene_id,
            line_index,
            line_type: line_type.to_string(),
            speaker_profile_id,
            text: text.to_string(),
            expression: None,
            pose: None,
            position: None,
            presentation_json: json!({}),
            text_anchor_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn choice_model(
        id: i64,
        line_id: i64,
        option_index: i32,
        option_text: &str,
        target_scene_id: i64,
        now: NaiveDateTime,
    ) -> galgame_choice::Model {
        galgame_choice::Model {
            id,
            line_id,
            option_index,
            option_text: option_text.to_string(),
            target_scene_id: Some(target_scene_id),
            created_at: now,
        }
    }

    fn media_summary(
        binding_id: i64,
        asset_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        file_path: &str,
    ) -> GalgameMediaBindingSummary {
        let media_kind = if role == MEDIA_BINDING_ROLE_BGM || role == MEDIA_BINDING_ROLE_SFX {
            MEDIA_KIND_AUDIO
        } else {
            MEDIA_KIND_IMAGE
        };
        media_summary_with_kind(
            binding_id,
            asset_id,
            target_kind,
            target_id,
            role,
            media_kind,
            file_path,
        )
    }

    fn media_summary_with_kind(
        binding_id: i64,
        asset_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        media_kind: &str,
        file_path: &str,
    ) -> GalgameMediaBindingSummary {
        let now = fixed_time();
        GalgameMediaBindingSummary {
            binding: media_binding::Model {
                id: binding_id,
                media_asset_id: asset_id,
                target_kind: target_kind.to_string(),
                target_id,
                role: role.to_string(),
                created_at: now,
            },
            asset: media_asset::Model {
                id: asset_id,
                project_id: 10,
                root_project_id: 1,
                source_snapshot_id: Some(20),
                source_chapter_id: Some(30),
                source_snapshot_chapter_id: Some(40),
                media_kind: media_kind.to_string(),
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
                file_path: Some(file_path.to_string()),
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                file_size: None,
                checksum: None,
                status: MEDIA_STATUS_COMPLETED.to_string(),
                error_message: None,
                created_at: now,
                updated_at: now,
            },
        }
    }

    fn portrait_model(
        id: i64,
        profile_id: i64,
        expression: Option<&str>,
        pose: Option<&str>,
        is_primary: bool,
        image_path: &str,
    ) -> character_portrait::Model {
        character_portrait::Model {
            id,
            profile_id,
            project_id: 1,
            portrait_type: "full_body".to_string(),
            image_path: image_path.to_string(),
            prompt: "portrait".to_string(),
            expression: expression.map(str::to_string),
            pose: pose.map(str::to_string),
            is_primary,
            created_at: fixed_time(),
        }
    }

    fn portrait_expression_spec(
        id: i64,
        target_entity_id: Option<i64>,
        params_json: serde_json::Value,
    ) -> generation_spec::Model {
        let now = fixed_time();
        generation_spec::Model {
            id,
            project_id: 10,
            root_project_id: 1,
            source_context_kind: Some("snapshot_backed".to_string()),
            source_snapshot_id: Some(20),
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            canvas_text_asset_id: None,
            target_kind: GALGAME_TARGET_PORTRAIT_EXPRESSION.to_string(),
            target_entity_kind: GALGAME_ENTITY_CHARACTER_PROFILE.to_string(),
            target_entity_id,
            status: "draft".to_string(),
            source_version_hash: "hash".to_string(),
            params_json,
            created_at: now,
            updated_at: now,
        }
    }

    fn background_visual_spec(
        id: i64,
        target_entity_id: Option<i64>,
        source_chapter_id: Option<i64>,
        source_snapshot_chapter_id: Option<i64>,
        params_json: serde_json::Value,
    ) -> generation_spec::Model {
        visual_spec(
            id,
            GALGAME_TARGET_BACKGROUND,
            GALGAME_ENTITY_SCENE,
            target_entity_id,
            source_chapter_id,
            source_snapshot_chapter_id,
            params_json,
        )
    }

    fn cg_visual_spec(
        id: i64,
        target_entity_id: Option<i64>,
        source_chapter_id: Option<i64>,
        source_snapshot_chapter_id: Option<i64>,
        params_json: serde_json::Value,
    ) -> generation_spec::Model {
        visual_spec(
            id,
            GALGAME_TARGET_CG,
            GALGAME_ENTITY_LINE,
            target_entity_id,
            source_chapter_id,
            source_snapshot_chapter_id,
            params_json,
        )
    }

    fn visual_spec(
        id: i64,
        target_kind: &str,
        target_entity_kind: &str,
        target_entity_id: Option<i64>,
        source_chapter_id: Option<i64>,
        source_snapshot_chapter_id: Option<i64>,
        params_json: serde_json::Value,
    ) -> generation_spec::Model {
        let now = fixed_time();
        generation_spec::Model {
            id,
            project_id: 10,
            root_project_id: 1,
            source_context_kind: Some("snapshot_backed".to_string()),
            source_snapshot_id: Some(20),
            source_chapter_id,
            source_snapshot_chapter_id,
            canvas_text_asset_id: None,
            target_kind: target_kind.to_string(),
            target_entity_kind: target_entity_kind.to_string(),
            target_entity_id,
            status: "draft".to_string(),
            source_version_hash: "hash".to_string(),
            params_json,
            created_at: now,
            updated_at: now,
        }
    }

    fn prompt_engine() -> TeraPromptEngine {
        let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
        TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
    }

    fn background_target_context() -> GalgameTargetContext {
        GalgameTargetContext {
            project_id: 10,
            root_project_id: 1,
            source_snapshot_id: 20,
            source_chapter_id: 30,
            source_snapshot_chapter_id: 40,
            scene_id: 100,
            line_id: None,
        }
    }

    fn cg_target_context() -> GalgameTargetContext {
        GalgameTargetContext {
            line_id: Some(1000),
            ..background_target_context()
        }
    }

    fn write_fixture_file(data_dir: &Path, relative_path: &str, bytes: &[u8]) {
        let path = data_dir.join(relative_path);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        std::fs::write(path, bytes).expect("write fixture");
    }

    fn assert_zip_text_contains(
        zip: &mut ZipArchive<Cursor<Vec<u8>>>,
        path: &str,
        needles: &[&str],
    ) {
        let mut text = String::new();
        zip.by_name(path)
            .unwrap_or_else(|err| panic!("missing zip text entry {path}: {err}"))
            .read_to_string(&mut text)
            .unwrap_or_else(|err| panic!("read zip text entry {path}: {err}"));
        for needle in needles {
            assert!(
                text.contains(needle),
                "zip entry {path} did not contain {needle:?}; content:\n{text}"
            );
        }
    }

    fn assert_zip_bytes(zip: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str, expected: &[u8]) {
        let mut bytes = Vec::new();
        zip.by_name(path)
            .unwrap_or_else(|err| panic!("missing zip binary entry {path}: {err}"))
            .read_to_end(&mut bytes)
            .unwrap_or_else(|err| panic!("read zip binary entry {path}: {err}"));
        assert_eq!(bytes, expected, "zip entry bytes differ: {path}");
    }

    fn valid_scene_segmentation_checkpoint_output() -> Value {
        json!({
            "scenes": [
                {
                    "index": 1,
                    "title": "雨夜重逢",
                    "location": "车站",
                    "time_of_day": "夜晚",
                    "participants": ["林晓雨"],
                    "summary": "两人重逢",
                    "source_span_ids": ["ch1:ta1"]
                }
            ]
        })
    }

    fn valid_script_adaptation_checkpoint_output() -> Value {
        json!({
            "scene_lines": [
                {
                    "scene_index": 1,
                    "lines": [
                        {
                            "index": 1,
                            "line_type": "dialogue",
                            "speaker_name": "林晓雨",
                            "text": "你终于来了。",
                            "source_span_ids": ["ch1:ta1"]
                        }
                    ]
                }
            ]
        })
    }

    fn fixed_time() -> NaiveDateTime {
        chrono::DateTime::from_timestamp(1_700_000_000, 0)
            .expect("valid timestamp")
            .naive_utc()
    }
}
