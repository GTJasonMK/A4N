use std::collections::{BTreeSet, HashMap};

use a4n_core::{
    dependency_conflict::DownstreamConflictStrategy,
    error::AppError,
    models::{
        ai::video_gen_config,
        chapter::{chapter, chapter_video_prompt},
        generation::workflow_run,
    },
    prompt_language::PromptOutputLanguage,
    source_spans::source_text_hash as source_content_sha256,
    video_execution::{
        VideoSegmentExecutionRecipeV1, video_segment_recipes_for_plan_from_metadata,
    },
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    workflow::{GetWorkflowRunInput, WorkflowRunDetailSummary, WorkflowRunFinalizeOutcome},
};

use super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSource,
    ChapterVideoSourceContext, UpdateChapterVideoPromptInput as DbUpdate, VideoPromptChunk,
    VideoPromptDownstreamDependencies, VideoPromptOptimizationInput, VideoPromptProgress,
    VideoPromptProgressUpdate, duration_capability_for_config,
    load_previous_shared_segment_recipe_state, materialize_video_segment_plan_draft_recipes,
    update_video_segment_plan_draft_recipe_checkpoint,
    validate_prompt_options_with_reference_limit, validate_video_duration_for_config_secs,
};

const VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
const VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
const VIDEO_WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
const VIDEO_WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
const VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN: &str = "shared_segment_plan";
const WORKFLOW_MODES: &[&str] = &[
    VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
    VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_STORYBOARD,
];
const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;

pub type GeneratePromptOptionsInput = a4n_api_contract::video::VideoPromptGenerationOptions;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoPromptInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(skip)]
    pub active_task_id: Option<String>,
    #[serde(skip)]
    pub root_project_id: Option<i64>,
    pub options: GeneratePromptOptionsInput,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterVideoPromptSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: Option<String>,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub shot_number: i32,
    pub title: Option<String>,
    pub scene_summary: Option<String>,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<String>,
    pub reference_image_ids_json: Option<Value>,
    pub status: String,
    pub generated_video_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterVideoPromptInput {
    pub visual_prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub motion_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub negative_prompt: Option<Option<String>>,
    #[serde(default)]
    pub segment_recipe: Option<VideoSegmentExecutionRecipeV1>,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadVideoSegmentPlanDraftInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoSegmentPlanDraftRecipeInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_run_id: i64,
    pub source_shot_id: String,
    pub segment_index_in_shot: i32,
    pub recipe: VideoSegmentExecutionRecipeV1,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeChapterVideoPromptInput {
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

pub struct VideoPromptApplicationContext<'a> {
    pub runtime_video_config: Option<&'a video_gen_config::Model>,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

#[async_trait]
pub trait VideoPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_media(
        &self,
        image_ids: &[i64],
        video_ids: &[i64],
    ) -> Result<(), AppError>;
}

#[derive(Clone, Copy)]
enum PromptGenerationStage {
    Frame,
    Segment,
    Video,
}

struct GenerationSpec {
    operation: &'static str,
    empty_content_message: &'static str,
    running_message: &'static str,
    workflow_kind: &'static str,
    workflow_step_key: &'static str,
}

struct PreparedGeneration {
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    source: ChapterVideoSource,
    variables: HashMap<String, Value>,
    options: ChapterVideoPromptOptions,
    workflow_run_id: i64,
}

struct VideoPromptMutationPlan {
    allow_downstream: bool,
    delete_dependencies: Option<VideoPromptDownstreamDependencies>,
}

struct VideoSegmentPlanDraftContext {
    run: workflow_run::Model,
    style: String,
    prompt_output_language: String,
    regenerate_existing: bool,
}

impl OrchestratorContext {
    pub async fn generate_chapter_video_frame_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Frame,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    pub async fn generate_chapter_video_segment_plan_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Segment,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    pub async fn generate_chapter_video_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Video,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    async fn generate_chapter_video_prompt_stage(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        stage: PromptGenerationStage,
        mut progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        report_progress(&mut progress, 0, 100, "正在检查章节和生成参数")?;
        check_cancelled(&cancel_token)?;
        let spec = generation_spec(stage);
        let prepared = self
            .prepare_video_prompt_generation(context.runtime_video_config, input, stage, spec)
            .await?;
        let result = self
            .execute_video_prompt_generation_stage(
                context.prompt_renderer,
                &prepared,
                stage,
                &mut progress,
                chunks,
                &cancel_token,
            )
            .await;
        let rows = self
            .finalize_video_prompt_generation(prepared.workflow_run_id, result)
            .await?;
        Ok(rows.iter().map(prompt_to_summary).collect())
    }

    async fn prepare_video_prompt_generation(
        &self,
        runtime_video_config: Option<&video_gen_config::Model>,
        input: GenerateChapterVideoPromptInput,
        stage: PromptGenerationStage,
        spec: GenerationSpec,
    ) -> Result<PreparedGeneration, AppError> {
        let GenerateChapterVideoPromptInput {
            user_id,
            project_id,
            chapter_id,
            active_task_id,
            root_project_id,
            mut options,
        } = input;
        let active_task_id = active_task_id.ok_or_else(|| AppError::InvalidState {
            current: "video_prompt_generation_without_task_owner".to_string(),
            operation: "启动视频提示词工作流".to_string(),
        })?;
        let root_project_id = root_project_id.ok_or_else(|| AppError::InvalidState {
            current: "video_prompt_generation_without_root_project".to_string(),
            operation: "启动视频提示词工作流".to_string(),
        })?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let source_bundle = self
            .project_chapter_video_source(project_id, chapter_id, spec.operation, true)
            .await?;
        if source_bundle.source.content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                spec.empty_content_message.to_string(),
            ));
        }
        if self
            .has_running_chapter_video_prompts(project_id, chapter_id)
            .await?
        {
            return Err(AppError::InvalidState {
                current: "video_prompt_running".to_string(),
                operation: spec.running_message.to_string(),
            });
        }
        let segment_recipe = self
            .resolve_segment_recipe_options(project_id, chapter_id, stage, &mut options)
            .await?;
        let workflow_mode = ensure_video_workflow_mode_for_stage(&options, stage)?;
        let mut options = validate_generate_prompt_options(options, &workflow_mode, None)?;
        options.segment_recipe = segment_recipe;
        if workflow_mode != VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
            let config = self.effective_video_config(runtime_video_config).await?;
            apply_duration_capability(&config, &mut options)?;
        }
        self.verify_video_reference_image_access(
            user_id,
            project_id,
            chapter_id,
            &options.reference_image_ids,
        )
        .await?;
        let variables = build_video_prompt_vars(
            project_id,
            chapter_id,
            &source_bundle.chapter,
            &source_bundle.source,
            &options,
        )?;
        let workflow_run = self
            .begin_video_prompt_workflow_validated(
                spec.workflow_kind,
                project_id,
                chapter_id,
                &workflow_mode,
                spec.workflow_step_key,
                &source_bundle.source,
                &options,
                &active_task_id,
                root_project_id,
            )
            .await?;
        options.workflow_run_id = Some(workflow_run.id);
        Ok(PreparedGeneration {
            project_id,
            chapter_id,
            workflow_mode,
            source: source_bundle.source,
            variables,
            options,
            workflow_run_id: workflow_run.id,
        })
    }

    async fn resolve_segment_recipe_options(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: PromptGenerationStage,
        options: &mut GeneratePromptOptionsInput,
    ) -> Result<Option<VideoSegmentExecutionRecipeV1>, AppError> {
        if matches!(stage, PromptGenerationStage::Segment) {
            if options.segment_plan_prompt_id.is_some() || options.target_shot_number.is_some() {
                return Err(AppError::InvalidArgument(
                    "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
                ));
            }
            return Ok(None);
        }
        let target = options.target_shot_number.ok_or_else(|| {
            AppError::InvalidArgument("片段卡片生成必须明确提供 target_shot_number".to_string())
        })?;
        let plan_id = options.segment_plan_prompt_id.ok_or_else(|| {
            AppError::InvalidArgument("片段卡片生成必须明确提供 segment_plan_prompt_id".to_string())
        })?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, plan_id)
            .await?;
        let plan_style = plan
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let recipe =
            video_segment_recipes_for_plan_from_metadata(plan.metadata_json.as_ref(), plan_style)
                .map_err(AppError::InvalidArgument)?
                .into_iter()
                .find(|recipe| recipe.segment_number == target)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("当前片段规划缺少片段 {target} 的执行方案"))
                })?;
        if recipe.workflow_mode.as_str() == "reference_media" {
            return Err(AppError::InvalidArgument(
                "参考媒体方案应直接使用参考媒体生成入口，不能生成普通帧或视频提示词".to_string(),
            ));
        }
        if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == VIDEO_WORKFLOW_MODE_CHAT_VIDEO
        {
            return Err(AppError::InvalidArgument(
                "Chat 方案不需要帧提示词，请直接生成当前片段的最终提示词".to_string(),
            ));
        }
        if matches!(stage, PromptGenerationStage::Video) {
            recipe
                .validate_generation_ready()
                .map_err(AppError::InvalidArgument)?;
        }
        options.workflow_mode = Some(recipe.workflow_mode.as_str().to_string());
        options.style.clone_from(&recipe.style);
        options.prompt_output_language = Some(recipe.prompt_output_language.clone());
        options.prompt_batch_size = if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        {
            2
        } else {
            1
        };
        options.target_shot_number = Some(recipe.segment_number);
        options.duration_secs = recipe.duration_secs;
        options
            .duration_secs_options
            .clone_from(&recipe.duration_secs_options);
        options.aspect_ratio.clone_from(&recipe.aspect_ratio);
        options
            .reference_image_ids
            .clone_from(&recipe.reference_image_ids);
        options
            .video_library_image_ids
            .clone_from(&recipe.video_library_image_ids);
        Ok(Some(recipe))
    }

    async fn execute_video_prompt_generation_stage(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        prepared: &PreparedGeneration,
        stage: PromptGenerationStage,
        progress: &mut Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: &CancellationToken,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        report_progress(
            progress,
            10,
            100,
            generation_prepare_message(&prepared.workflow_mode, stage),
        )?;
        check_cancelled(cancel_token)?;
        let source = ChapterVideoSourceContext {
            content: &prepared.source.content,
            summary: prepared.source.summary.as_deref(),
            analysis: prepared.source.analysis.as_ref(),
        };
        let input = ChapterVideoPromptStageInput {
            project_id: prepared.project_id,
            chapter_id: prepared.chapter_id,
            workflow_mode: &prepared.workflow_mode,
            source,
            prompt_renderer,
            variables: &prepared.variables,
            options: prepared.options.clone(),
            cancel_token: Some(cancel_token),
        };
        match stage {
            PromptGenerationStage::Frame => {
                self.generate_chapter_frame_prompts_for_mode(input, chunks, progress.take())
                    .await
            }
            PromptGenerationStage::Segment => {
                self.generate_chapter_segment_plan(input, chunks, progress.take())
                    .await
            }
            PromptGenerationStage::Video => {
                self.generate_chapter_video_prompts_for_mode(input, chunks, progress.take())
                    .await
            }
        }
    }

    async fn finalize_video_prompt_generation(
        &self,
        workflow_run_id: i64,
        result: Result<Vec<chapter_video_prompt::Model>, AppError>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        match result {
            Ok(rows) => {
                self.complete_workflow_run_validated(workflow_run_id)
                    .await?;
                Ok(rows)
            }
            Err(error) => {
                let outcome = if matches!(error, AppError::Cancelled) {
                    WorkflowRunFinalizeOutcome::Cancelled
                } else {
                    WorkflowRunFinalizeOutcome::Failed
                };
                let message = error.to_string();
                self.finalize_workflow_run_outcome_validated(
                    workflow_run_id,
                    outcome,
                    Some(&message),
                )
                .await
                .map_err(|finalize_error| {
                    if matches!(&finalize_error, AppError::Cancelled) {
                        return AppError::Cancelled;
                    }
                    AppError::InvalidState {
                        current: message,
                        operation: format!("同步视频工作流状态失败: {finalize_error}"),
                    }
                })?;
                Err(error)
            }
        }
    }

    pub async fn list_chapter_video_prompts_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(project_id, chapter_id, "查看视频提示词", false)
            .await?;
        Ok(self
            .chapter_video_prompts_for_chapter(project_id, chapter_id)
            .await?
            .iter()
            .map(prompt_to_summary)
            .collect())
    }

    pub async fn load_video_segment_plan_draft_validated(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        let context = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                false,
            )
            .await?;
        self.materialize_video_segment_plan_draft_context(&context)
            .await?;
        self.workflow_run_detail(GetWorkflowRunInput {
            workflow_run_id: context.run.id,
            include_generation_runs: Some(false),
        })
        .await
    }

    pub async fn update_video_segment_plan_draft_recipe_validated(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        let context = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                true,
            )
            .await?;
        self.ensure_segment_plan_finalization_not_started(context.run.id)
            .await?;
        self.materialize_video_segment_plan_draft_context(&context)
            .await?;
        update_video_segment_plan_draft_recipe_checkpoint(
            self,
            context.run.id,
            &input.source_shot_id,
            input.segment_index_in_shot,
            &context.style,
            input.recipe,
        )
        .await?;
        self.ensure_segment_plan_finalization_not_started(context.run.id)
            .await?;
        self.workflow_run_detail(GetWorkflowRunInput {
            workflow_run_id: context.run.id,
            include_generation_runs: Some(false),
        })
        .await
    }

    async fn video_segment_plan_draft_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        workflow_run_id: i64,
        writable: bool,
    ) -> Result<VideoSegmentPlanDraftContext, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(
            project_id,
            chapter_id,
            if writable {
                "更新片段规划草稿执行方案"
            } else {
                "加载片段规划草稿执行方案"
            },
            writable,
        )
        .await?;
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        if run.workflow_kind != "video_segment_plan"
            || run.project_id != Some(project_id)
            || run.chapter_id != Some(chapter_id)
            || run.mode.as_deref() != Some(VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(AppError::InvalidArgument(format!(
                "片段规划 workflow {workflow_run_id} 与当前项目或章节不一致"
            )));
        }
        if run.status == "superseded" || (writable && run.status == "completed") {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: if writable {
                    "更新片段规划草稿执行方案".to_string()
                } else {
                    "加载片段规划草稿执行方案".to_string()
                },
            });
        }
        let fingerprint = run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_input_fingerprint_json".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?;
        if fingerprint.get("schema_version").and_then(Value::as_i64) != Some(1)
            || fingerprint.get("workflow_kind").and_then(Value::as_str)
                != Some("video_segment_plan")
            || fingerprint.get("project_id").and_then(Value::as_i64) != Some(project_id)
            || fingerprint.get("chapter_id").and_then(Value::as_i64) != Some(chapter_id)
            || fingerprint.get("workflow_mode").and_then(Value::as_str)
                != Some(VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(AppError::InvalidState {
                current: "invalid_input_fingerprint_json".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            });
        }
        let options = fingerprint
            .get("options")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_input_fingerprint_options".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?;
        let style = options
            .get("style")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: "invalid_input_fingerprint_style".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?
            .to_string();
        let prompt_output_language = options
            .get("prompt_output_language")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "zh_cn" | "en"))
            .ok_or_else(|| AppError::InvalidState {
                current: "invalid_input_fingerprint_prompt_output_language".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?
            .to_string();
        let regenerate_existing = options
            .get("regenerate_existing")
            .and_then(Value::as_bool)
            .ok_or_else(|| AppError::InvalidState {
                current: "invalid_input_fingerprint_regenerate_existing".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?;
        Ok(VideoSegmentPlanDraftContext {
            run,
            style,
            prompt_output_language,
            regenerate_existing,
        })
    }

    async fn materialize_video_segment_plan_draft_context(
        &self,
        context: &VideoSegmentPlanDraftContext,
    ) -> Result<(), AppError> {
        let previous = load_previous_shared_segment_recipe_state(
            self,
            context
                .run
                .project_id
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_project_id".to_string(),
                    operation: format!("读取片段规划 workflow {}", context.run.id),
                })?,
            context
                .run
                .chapter_id
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_chapter_id".to_string(),
                    operation: format!("读取片段规划 workflow {}", context.run.id),
                })?,
            context.regenerate_existing,
        )
        .await?;
        materialize_video_segment_plan_draft_recipes(
            self,
            context.run.id,
            &context.style,
            &context.prompt_output_language,
            previous
                .as_ref()
                .map(|(segments, recipes)| (segments.as_slice(), recipes.as_slice())),
        )
        .await
    }

    async fn ensure_segment_plan_finalization_not_started(
        &self,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        if self
            .workflow_checkpoint_model_by_step(workflow_run_id, "video_segment_plan.final")
            .await?
            .is_some()
        {
            return Err(AppError::InvalidState {
                current: "segment_plan_finalizing".to_string(),
                operation: "更新片段规划草稿执行方案，请在正式规划加载后重试".to_string(),
            });
        }
        Ok(())
    }

    pub async fn update_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(
            prompt.project_id,
            prompt.chapter_id,
            "更新视频提示词",
            true,
        )
        .await?;
        if let Some(recipe) = input.segment_recipe.clone() {
            return self
                .update_video_segment_recipe(prompt, input, recipe, user_id)
                .await;
        }
        let plan = self
            .prepare_mutation_plan(
                prompt_id,
                input.downstream_strategy,
                "更新视频提示词",
                "该视频提示词",
            )
            .await?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        let row = self
            .update_chapter_video_prompt_with_policy(
                prompt_id,
                DbUpdate {
                    visual_prompt: input.visual_prompt,
                    motion_prompt: input.motion_prompt,
                    negative_prompt: input.negative_prompt,
                    ..Default::default()
                },
                plan.allow_downstream,
            )
            .await?;
        Ok(prompt_to_summary(&row))
    }

    pub(in crate::video) async fn current_shared_segment_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let current_source_hash = source_content_sha256(
            &self
                .project_chapter_video_source(project_id, chapter_id, "读取当前共享片段规划", false)
                .await?
                .source
                .content,
        );
        latest_current_shared_segment_plan_from_rows(
            self.chapter_video_prompts_for_chapter(project_id, chapter_id)
                .await?,
            &current_source_hash,
        )
        .ok_or_else(|| AppError::InvalidArgument("当前章节没有可用的共享片段规划".to_string()))
    }

    pub(in crate::video) async fn require_current_shared_segment_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        if prompt.project_id != project_id
            || prompt.chapter_id != chapter_id
            || prompt.prompt_type != "frame_segment_plan"
            || prompt.workflow_mode != "shared_segment_plan"
            || prompt.status != "completed"
            || prompt
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("video_execution_identity"))
                .is_none()
        {
            return Err(AppError::InvalidArgument(
                "目标片段规划不是当前章节已完成的共享片段规划".to_string(),
            ));
        }
        let current = self
            .current_shared_segment_plan(project_id, chapter_id)
            .await?;
        let current_source_hash = video_prompt_source_content_hash(current.metadata_json.as_ref())
            .ok_or_else(|| AppError::InvalidArgument("当前共享片段规划缺少正文身份".to_string()))?;
        if video_prompt_source_content_hash(prompt.metadata_json.as_ref())
            != Some(current_source_hash)
        {
            return Err(AppError::InvalidArgument(
                "目标片段规划对应的章节正文已变化，请重新生成片段规划".to_string(),
            ));
        }
        if current.id != prompt.id {
            return Err(AppError::InvalidArgument(
                "目标片段规划不是当前规划，请刷新后重试".to_string(),
            ));
        }
        Ok(prompt)
    }

    async fn update_video_segment_recipe(
        &self,
        prompt: chapter_video_prompt::Model,
        input: UpdateChapterVideoPromptInput,
        mut recipe: VideoSegmentExecutionRecipeV1,
        user_id: i64,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        if input.visual_prompt.is_some()
            || input.motion_prompt.is_some()
            || input.negative_prompt.is_some()
        {
            return Err(AppError::InvalidArgument(
                "更新片段执行方案时不能同时编辑提示词字段".to_string(),
            ));
        }
        let prompt = self
            .require_current_shared_segment_plan(prompt.project_id, prompt.chapter_id, prompt.id)
            .await?;
        let plan_style = prompt
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let current_recipes =
            video_segment_recipes_for_plan_from_metadata(prompt.metadata_json.as_ref(), plan_style)
                .map_err(AppError::InvalidArgument)?;
        if recipe.guidance_mode == a4n_core::video_execution::VideoSegmentGuidanceModeV1::Continuity
            && let Some(video_id) = recipe.continuity_source_video_id
        {
            let source_recipe = current_recipes
                .iter()
                .find(|candidate| {
                    candidate.segment_number < recipe.segment_number
                        && candidate.adopted_video_id == Some(video_id)
                })
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "续接来源视频 #{video_id} 必须是当前规划更早片段已采用的视频"
                    ))
                })?;
            let video = self
                .generated_video_by_id(video_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("视频 #{video_id} 不存在")))?;
            let plan_identity = prompt
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("video_execution_identity"))
                .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
            let expected_identity = super::prompt_generation::video_segment_execution_identity(
                plan_identity.clone(),
                Some(prompt.id),
                Some(source_recipe),
            )?;
            if video.project_id != prompt.project_id
                || video.chapter_id != Some(prompt.chapter_id)
                || video.status != "completed"
                || video
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_none_or(str::is_empty)
                || !super::prompt_records::video_execution_segment_identity_matches(
                    video.metadata_json.as_ref(),
                    &expected_identity,
                )
            {
                return Err(AppError::InvalidArgument(format!(
                    "续接来源视频 #{video_id} 已失效或不属于当前规划"
                )));
            }
        }
        let mut reference_ids = recipe.reference_image_ids.clone();
        reference_ids.extend(recipe.video_library_image_ids.iter().copied());
        reference_ids.extend(recipe.guidance_reference_image_ids.iter().copied());
        reference_ids.sort_unstable();
        reference_ids.dedup();
        self.verify_video_reference_image_access_json(
            user_id,
            prompt.project_id,
            prompt.chapter_id,
            Some(&json!(reference_ids)),
        )
        .await?;
        // adopted_video_id 只能通过采用视频入口修改；DB 会在指纹未变化时原子保留当前值。
        recipe.adopted_video_id = None;
        let row = self
            .update_chapter_video_prompt_with_policy(
                prompt.id,
                DbUpdate {
                    segment_recipe: Some(recipe),
                    ..Default::default()
                },
                true,
            )
            .await?;
        Ok(prompt_to_summary(&row))
    }

    pub async fn delete_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(
            prompt.project_id,
            prompt.chapter_id,
            "删除视频提示词",
            true,
        )
        .await?;
        let plan = self
            .prepare_mutation_plan(
                prompt_id,
                downstream_strategy,
                "删除视频提示词",
                "该视频提示词",
            )
            .await?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        self.delete_chapter_video_prompt_checked(prompt_id, plan.allow_downstream)
            .await
    }

    pub async fn optimize_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let plan = self
            .prepare_mutation_plan(
                prompt_id,
                input.downstream_strategy,
                "优化视频提示词",
                "该视频提示词",
            )
            .await?;
        self.optimize_video_prompt_validated(
            VideoPromptOptimizationInput {
                prompt,
                instruction: input.instruction,
                allow_downstream: plan.allow_downstream,
                delete_dependencies: plan.delete_dependencies,
            },
            prompt_renderer,
            cleanup,
        )
        .await
        .map(|row| prompt_to_summary(&row))
    }

    async fn prepare_mutation_plan(
        &self,
        prompt_id: i64,
        strategy: DownstreamConflictStrategy,
        operation: &str,
        subject: &str,
    ) -> Result<VideoPromptMutationPlan, AppError> {
        let dependencies = self.prompt_downstream_dependencies(prompt_id).await?;
        mutation_plan_from_dependencies(dependencies, strategy, operation, subject)
    }

    async fn apply_downstream_cleanup(
        &self,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
        plan: &VideoPromptMutationPlan,
    ) -> Result<(), AppError> {
        let Some(dependencies) = plan.delete_dependencies.as_ref() else {
            return Ok(());
        };
        cleanup
            .delete_downstream_media(&dependencies.image_ids, &dependencies.video_ids)
            .await?;
        if !dependencies.prompt_ids.is_empty() {
            self.delete_chapter_video_prompts_checked(&dependencies.prompt_ids, true)
                .await?;
        }
        Ok(())
    }

    async fn effective_video_config(
        &self,
        runtime_video_config: Option<&video_gen_config::Model>,
    ) -> Result<video_gen_config::Model, AppError> {
        match runtime_video_config {
            Some(config) => Ok(config.clone()),
            None => self.default_video_config().await,
        }
    }

    async fn verify_video_reference_image_access(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids: &[i64],
    ) -> Result<(), AppError> {
        if reference_image_ids.is_empty() {
            return Ok(());
        }
        let ids = json!(reference_image_ids);
        self.verify_video_reference_image_access_json(user_id, project_id, chapter_id, Some(&ids))
            .await
    }

    async fn verify_video_reference_image_access_json(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids: Option<&Value>,
    ) -> Result<(), AppError> {
        let project_ids = self
            .validated_video_reference_images(project_id, chapter_id, reference_image_ids)
            .await?
            .into_iter()
            .map(|image| image.project_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if project_ids.is_empty() {
            return Ok(());
        }
        self.verify_project_permissions(&project_ids, user_id, ProjectAccessRole::Editor)
            .await
    }
}

fn generation_spec(stage: PromptGenerationStage) -> GenerationSpec {
    match stage {
        PromptGenerationStage::Frame => GenerationSpec {
            operation: "生成边界帧提示词",
            empty_content_message: "章节正文为空，无法生成边界帧提示词",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成边界帧提示词",
            workflow_kind: "video_frame_prompts",
            workflow_step_key: "frame_prompts",
        },
        PromptGenerationStage::Segment => GenerationSpec {
            operation: "生成片段规划",
            empty_content_message: "章节正文为空，无法生成片段规划",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成片段规划",
            workflow_kind: "video_segment_plan",
            workflow_step_key: "segment_plan",
        },
        PromptGenerationStage::Video => GenerationSpec {
            operation: "生成视频提示词",
            empty_content_message: "章节正文为空，无法生成视频提示词",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成视频提示词",
            workflow_kind: "video_prompts",
            workflow_step_key: "video_prompts",
        },
    }
}

fn generation_prepare_message(workflow_mode: &str, stage: PromptGenerationStage) -> &'static str {
    if !matches!(stage, PromptGenerationStage::Video) {
        return "正在准备章节正文、素材和时长档位";
    }
    match workflow_mode {
        VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN => "正在准备章节正文、首帧和素材",
        VIDEO_WORKFLOW_MODE_CHAT_VIDEO => "正在准备章节正文和 Chat 上传素材",
        VIDEO_WORKFLOW_MODE_STORYBOARD => "正在准备章节正文、分镜图和素材",
        _ => "正在准备章节正文、边界帧和素材",
    }
}

fn ensure_video_workflow_mode(input: &GeneratePromptOptionsInput) -> Result<String, AppError> {
    let mode = input
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("视频提示词生成缺少 workflow_mode".to_string()))?;
    if !WORKFLOW_MODES.contains(&mode) {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词生成 workflow_mode 无效: {mode}"
        )));
    }
    Ok(mode.to_string())
}

fn ensure_video_workflow_mode_for_stage(
    input: &GeneratePromptOptionsInput,
    stage: PromptGenerationStage,
) -> Result<String, AppError> {
    let mode = ensure_video_workflow_mode(input)?;
    match stage {
        PromptGenerationStage::Segment if mode != VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN => {
            Err(AppError::InvalidArgument(
                "共享片段规划 workflow_mode 必须为 shared_segment_plan".to_string(),
            ))
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if mode == VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN =>
        {
            Err(AppError::InvalidArgument(
                "片段卡片生成必须使用卡片执行模式，不能使用 shared_segment_plan".to_string(),
            ))
        }
        _ => Ok(mode),
    }
}

fn validate_video_prompt_generation_options_for_stage(
    input: &GeneratePromptOptionsInput,
    stage: PromptGenerationStage,
) -> Result<(), AppError> {
    let workflow_mode = ensure_video_workflow_mode_for_stage(input, stage)?;
    match stage {
        PromptGenerationStage::Segment
            if input.segment_plan_prompt_id.is_some() || input.target_shot_number.is_some() =>
        {
            return Err(AppError::InvalidArgument(
                "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
            ));
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if input.segment_plan_prompt_id.is_none() || input.target_shot_number.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "片段卡片生成必须提供 segment_plan_prompt_id 和 target_shot_number".to_string(),
            ));
        }
        _ => {}
    }
    validate_generate_prompt_options(input.clone(), &workflow_mode, None).map(|_| ())
}

pub(crate) fn validate_video_segment_plan_generation_options(
    input: &GeneratePromptOptionsInput,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Segment)
}

pub(crate) fn validate_video_frame_prompt_generation_options(
    input: &GeneratePromptOptionsInput,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Frame)
}

pub(crate) fn validate_video_final_prompt_generation_options(
    input: &GeneratePromptOptionsInput,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Video)
}

fn validate_generate_prompt_options(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
    blank_style_placeholder: Option<&str>,
) -> Result<ChapterVideoPromptOptions, AppError> {
    let mut style = input.style.trim().to_string();
    if style.is_empty() {
        style = blank_style_placeholder
            .ok_or_else(|| AppError::InvalidArgument("视频提示词生成缺少 style".to_string()))?
            .to_string();
    }
    let prompt_output_language =
        PromptOutputLanguage::from_optional_code(input.prompt_output_language.as_deref())
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let reference_limit = if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        MAX_CHAT_REFERENCE_IMAGE_IDS
    } else {
        MAX_NON_CHAT_REFERENCE_IMAGE_IDS
    };
    validate_prompt_options_with_reference_limit(
        ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            prompt_batch_size: input.prompt_batch_size,
            segment_plan_prompt_id: input.segment_plan_prompt_id,
            target_shot_number: input.target_shot_number,
            segment_recipe: None,
            duration_secs: input.duration_secs,
            duration_secs_options: input.duration_secs_options,
            aspect_ratio: input.aspect_ratio,
            reference_image_ids: input.reference_image_ids,
            video_library_image_ids: input.video_library_image_ids,
            duration_capability: None,
            regenerate_existing: input.regenerate_existing,
            workflow_run_id: None,
        },
        reference_limit,
    )
}

fn apply_duration_capability(
    config: &video_gen_config::Model,
    options: &mut ChapterVideoPromptOptions,
) -> Result<(), AppError> {
    validate_video_duration_for_config_secs(config, options.duration_secs)
        .map_err(AppError::InvalidArgument)?;
    let capability = duration_capability_for_config(config).map_err(AppError::InvalidArgument)?;
    capability.validate_duration_options(&options.duration_secs_options)?;
    options.duration_capability = Some(capability);
    Ok(())
}

fn build_video_prompt_vars(
    project_id: i64,
    chapter_id: i64,
    chapter: &chapter::Model,
    source: &ChapterVideoSource,
    options: &ChapterVideoPromptOptions,
) -> Result<HashMap<String, Value>, AppError> {
    let mut variables = HashMap::new();
    variables.insert("project_id".into(), json!(project_id));
    variables.insert("chapter_id".into(), json!(chapter_id));
    variables.insert("chapter_number".into(), json!(chapter.chapter_number));
    variables.insert(
        "chapter_title".into(),
        json!(required_chapter_title(
            chapter.title.as_deref(),
            chapter.chapter_number,
        )?),
    );
    variables.insert("chapter_status".into(), json!(&chapter.status));
    variables.insert(
        "source_content_sha256".into(),
        json!(source_content_sha256(&source.content)),
    );
    if let Some(summary) = source
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        variables.insert("chapter_summary".into(), json!(summary));
    }
    if let Some(analysis) = source.analysis.as_ref() {
        variables.insert("chapter_analysis".into(), analysis.clone());
        if let Some(compressed) = analysis
            .get("summaries")
            .and_then(|value| value.get("compressed"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            variables.insert("chapter_analysis_summary".into(), json!(compressed));
        }
    }
    variables.insert("style".into(), json!(&options.style));
    variables.insert(
        "prompt_output_language".into(),
        json!(options.prompt_output_language.as_str()),
    );
    variables.insert("duration_secs".into(), json!(options.duration_secs));
    variables.insert(
        "duration_secs_options".into(),
        json!(&options.duration_secs_options),
    );
    if let Some(capability) = options.duration_capability.as_ref() {
        variables.insert(
            "video_duration_capability".into(),
            capability.to_prompt_json(),
        );
    }
    variables.insert("aspect_ratio".into(), json!(&options.aspect_ratio));
    variables.insert(
        "reference_image_ids".into(),
        json!(&options.reference_image_ids),
    );
    variables.insert(
        "video_library_image_ids".into(),
        json!(&options.video_library_image_ids),
    );
    Ok(variables)
}

fn required_chapter_title(title: Option<&str>, chapter_number: i32) -> Result<&str, AppError> {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "第 {chapter_number} 章缺少标题，无法生成视频提示词"
            ))
        })
}

fn video_prompt_source_content_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata?
        .get("video_execution_identity")?
        .get("source_content_sha256")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn latest_current_shared_segment_plan_from_rows(
    rows: Vec<chapter_video_prompt::Model>,
    current_source_hash: &str,
) -> Option<chapter_video_prompt::Model> {
    rows.into_iter()
        .filter(|row| {
            row.prompt_type == "frame_segment_plan"
                && row.workflow_mode == "shared_segment_plan"
                && row.status == "completed"
                && row
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("video_execution_identity"))
                    .is_some()
                && video_prompt_source_content_hash(row.metadata_json.as_ref())
                    == Some(current_source_hash)
        })
        .max_by_key(|row| row.id)
}

fn mutation_plan_from_dependencies(
    dependencies: VideoPromptDownstreamDependencies,
    strategy: DownstreamConflictStrategy,
    operation: &str,
    subject: &str,
) -> Result<VideoPromptMutationPlan, AppError> {
    if dependencies.is_empty() {
        return Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: None,
        });
    }
    match strategy {
        DownstreamConflictStrategy::Reject => Err(AppError::InvalidState {
            current: format!(
                "{subject}已有 {} 个下游派生产物",
                dependencies.prompt_ids.len()
                    + dependencies.image_ids.len()
                    + dependencies.video_ids.len()
            ),
            operation: format!("{operation}；请选择删除下游后继续，或保留下游继续"),
        }),
        DownstreamConflictStrategy::KeepDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: true,
            delete_dependencies: None,
        }),
        DownstreamConflictStrategy::DeleteDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: Some(dependencies),
        }),
    }
}

fn prompt_to_summary(prompt: &chapter_video_prompt::Model) -> ChapterVideoPromptSummary {
    ChapterVideoPromptSummary {
        id: prompt.id,
        project_id: prompt.project_id,
        chapter_id: prompt.chapter_id,
        style: prompt.style.clone(),
        prompt_type: prompt.prompt_type.clone(),
        workflow_mode: prompt.workflow_mode.clone(),
        shot_number: prompt.shot_number,
        title: prompt.title.clone(),
        scene_summary: prompt.scene_summary.clone(),
        visual_prompt: prompt.visual_prompt.clone(),
        motion_prompt: prompt.motion_prompt.clone(),
        negative_prompt: prompt.negative_prompt.clone(),
        duration_secs: prompt.duration_secs,
        aspect_ratio: prompt.aspect_ratio.clone(),
        reference_image_ids_json: prompt.reference_image_ids_json.clone(),
        status: prompt.status.clone(),
        generated_video_id: prompt.generated_video_id,
        metadata_json: prompt.metadata_json.clone(),
        created_at: Some(prompt.created_at.to_string()),
        updated_at: Some(prompt.updated_at.to_string()),
    }
}

fn report_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(VideoPromptProgressUpdate::progress(current, total, message))?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), AppError> {
    if cancel_token.is_cancelled() {
        Err(AppError::Cancelled)
    } else {
        Ok(())
    }
}

fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment_plan_options(workflow_mode: &str) -> GeneratePromptOptionsInput {
        GeneratePromptOptionsInput {
            workflow_mode: Some(workflow_mode.to_string()),
            segment_plan_prompt_id: None,
            style: "cinematic".to_string(),
            prompt_output_language: Some("zh_cn".to_string()),
            prompt_batch_size: 1,
            target_shot_number: None,
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
            video_library_image_ids: Vec::new(),
            regenerate_existing: false,
        }
    }

    #[test]
    fn segment_plan_accepts_only_shared_mode_before_task_creation() {
        assert!(
            validate_video_segment_plan_generation_options(&segment_plan_options(
                VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            ))
            .is_ok()
        );
        let error = validate_video_segment_plan_generation_options(&segment_plan_options(
            VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
        ))
        .unwrap_err();
        assert!(error.to_string().contains("必须为 shared_segment_plan"));
    }

    #[test]
    fn mutation_strategy_keeps_one_source_of_truth() {
        let dependencies = VideoPromptDownstreamDependencies {
            prompt_ids: vec![1],
            image_ids: vec![2],
            video_ids: vec![3],
        };
        let plan = mutation_plan_from_dependencies(
            dependencies,
            DownstreamConflictStrategy::DeleteDownstream,
            "更新视频提示词",
            "该视频提示词",
        )
        .unwrap();
        assert!(!plan.allow_downstream);
        assert_eq!(plan.delete_dependencies.unwrap().video_ids, vec![3]);
    }

    #[test]
    fn shared_plan_source_hash_requires_execution_identity() {
        let current = json!({
            "source": { "content_sha256": "legacy" },
            "video_execution_identity": { "source_content_sha256": "current" },
        });
        assert_eq!(
            video_prompt_source_content_hash(Some(&current)),
            Some("current")
        );

        let legacy = json!({
            "source": { "content_sha256": "legacy" },
        });
        assert_eq!(video_prompt_source_content_hash(Some(&legacy)), None);
    }

    #[test]
    fn current_shared_plan_ignores_newer_stale_content() {
        let plan = |id: i64, source_hash: &str| chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 2,
            style: Some("cinematic".to_string()),
            prompt_type: "frame_segment_plan".to_string(),
            workflow_mode: "shared_segment_plan".to_string(),
            shot_number: 0,
            title: None,
            scene_summary: None,
            visual_prompt: "{}".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "completed".to_string(),
            generated_video_id: None,
            metadata_json: Some(json!({
                "video_execution_identity": {
                    "source_content_sha256": source_hash
                }
            })),
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
        };

        let current = latest_current_shared_segment_plan_from_rows(
            vec![plan(41, "current"), plan(42, "stale")],
            "current",
        )
        .unwrap();

        assert_eq!(current.id, 41);
    }
}
