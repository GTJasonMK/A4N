use a4n_error::AppError;
use std::collections::{BTreeSet, HashMap};

pub use a4n_api_contract::video::{
    ChapterVideoPromptSummary, LoadVideoSegmentPlanDraftInput, OptimizeChapterVideoPromptInput,
    UpdateChapterVideoPromptInput, UpdateVideoSegmentPlanDraftRecipeInput,
};
use a4n_api_contract::video::{VideoPromptDownstreamCleanupPort, VideoPromptGenerationOptions};
use a4n_db::video_prompt::ChapterVideoPromptPatch;
use a4n_db_models::models::chapter::{chapter, chapter_video_prompt};
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::{DownstreamConflictStrategy, source_text_hash as source_content_sha256};
use a4n_prompt::rendering::PromptRenderer;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_contract::{
    VideoSegmentExecutionRecipeV1, VideoSegmentWorkflowModeV1, resolve_video_segment_number,
    video_segment_recipes_for_plan_from_metadata,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    workflow::{
        WorkflowRunSummary, workflow_run_is_running_or_resumable, workflow_run_summary_to_api,
    },
};

use super::prompt_context::validate_prompt_options_with_reference_limit;
use super::prompt_optimization::VideoPromptOptimizationInput;
use super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSource,
    ChapterVideoSourceContext, VideoPromptChunk, VideoPromptDownstreamDependencies,
    VideoPromptProgress, VideoPromptProgressUpdate, duration_capability_for_provider_config,
    load_video_segment_plan_draft_checkpoints, load_video_segment_plan_draft_recipe_state,
    update_video_segment_plan_draft_recipe_checkpoint,
    validate_video_duration_for_provider_config_secs, video_db_error,
};

const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoPromptInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(skip)]
    pub active_task_id: Option<String>,
    #[serde(skip)]
    pub root_project_id: Option<i64>,
    #[serde(skip)]
    pub retry_of_task_id: Option<String>,
    pub options: VideoPromptGenerationOptions,
}

pub struct VideoSegmentPlanDraftState {
    pub run: a4n_api_contract::workflow::WorkflowRunSummary,
    pub segments: Vec<a4n_video_contract::VideoSegmentPlanV2>,
    pub recipes: Vec<VideoSegmentExecutionRecipeV1>,
    pub completed_shot_count: usize,
    pub total_shot_count: usize,
}

pub struct VideoPromptApplicationContext<'a> {
    pub runtime_video_config: Option<&'a VideoProviderConfig>,
    pub prompt_renderer: &'a dyn PromptRenderer,
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
    run: WorkflowRunSummary,
    style: String,
}

impl OrchestratorContext {
    pub(in crate::video) async fn generate_chapter_video_frame_prompts_validated(
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

    pub(in crate::video) async fn generate_chapter_video_segment_plan_validated(
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

    pub(in crate::video) async fn generate_chapter_video_prompts_validated(
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
            .finalize_workflow_result(prepared.workflow_run_id, result, "视频提示词生成")
            .await?;
        Ok(rows.iter().map(prompt_to_summary).collect())
    }

    async fn prepare_video_prompt_generation(
        &self,
        runtime_video_config: Option<&VideoProviderConfig>,
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
            retry_of_task_id,
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
        let (source_bundle, segment_recipe) = futures::join!(
            self.project_chapter_video_source(project_id, chapter_id, spec.operation, true),
            self.resolve_segment_recipe_options(project_id, chapter_id, stage, &mut options),
        );
        let source_bundle = source_bundle?;
        if source_bundle.source.content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                spec.empty_content_message.to_string(),
            ));
        }
        let segment_recipe = segment_recipe?;
        let workflow_mode = ensure_video_workflow_mode_for_stage(&options, stage)?;
        let mut options = validate_generate_prompt_options(options, &workflow_mode, None)?;
        options.target_shot_number = segment_recipe.as_ref().map(|recipe| recipe.segment_number);
        options.segment_recipe = segment_recipe;
        let (config, reference_access) = futures::join!(
            async {
                if workflow_mode != chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO {
                    self.effective_video_config(runtime_video_config)
                        .await
                        .map(Some)
                } else {
                    Ok::<Option<VideoProviderConfig>, AppError>(None)
                }
            },
            self.verify_video_reference_image_access(
                user_id,
                project_id,
                chapter_id,
                &options.reference_image_ids,
            ),
        );
        let config = config?;
        reference_access?;
        if let Some(config) = config {
            apply_duration_capability(&config, &mut options)?;
        }
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
                retry_of_task_id.as_deref(),
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
        options: &mut VideoPromptGenerationOptions,
    ) -> Result<Option<VideoSegmentExecutionRecipeV1>, AppError> {
        if matches!(stage, PromptGenerationStage::Segment) {
            if options.segment_plan_prompt_id.is_some() || options.target_segment.is_some() {
                return Err(AppError::InvalidArgument(
                    "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
                ));
            }
            return Ok(None);
        }
        if options.target_narrative_unit_index.is_some() {
            return Err(AppError::InvalidArgument(
                "target_narrative_unit_index 只能用于生成片段规划".to_string(),
            ));
        }
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
        let target_segment = options.target_segment.as_ref().ok_or_else(|| {
            AppError::InvalidArgument("片段卡片生成必须明确提供稳定片段目标".to_string())
        })?;
        let target = resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            &target_segment.source_shot_id,
            target_segment.segment_index_in_shot,
        )
        .map_err(AppError::InvalidArgument)?;
        let recipe =
            video_segment_recipes_for_plan_from_metadata(plan.metadata_json.as_ref(), plan_style)
                .map_err(AppError::InvalidArgument)?
                .into_iter()
                .find(|recipe| recipe.segment_number == target)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("当前片段规划缺少片段 {target} 的执行方案"))
                })?;
        if recipe.workflow_mode == VideoSegmentWorkflowModeV1::ReferenceMedia {
            return Err(AppError::InvalidArgument(
                "参考媒体方案应直接使用参考媒体生成入口，不能生成普通帧或视频提示词".to_string(),
            ));
        }
        if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
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
        options
            .prompt_output_language
            .clone_from(&recipe.prompt_output_language);
        options.prompt_batch_size = if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
        {
            2
        } else {
            1
        };
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
    ) -> Result<VideoSegmentPlanDraftState, AppError> {
        let VideoSegmentPlanDraftContext { run, style } = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                "加载片段规划草稿执行方案",
            )
            .await?;
        let checkpoints = load_video_segment_plan_draft_checkpoints(self, run.id).await?;
        let (segments, recipes, completed_shot_count, total_shot_count) = if checkpoints.is_empty()
        {
            (Vec::new(), Vec::new(), 0, 0)
        } else {
            load_video_segment_plan_draft_recipe_state(&checkpoints, &style)?
        };
        Ok(VideoSegmentPlanDraftState {
            run: workflow_run_summary_to_api(run),
            segments,
            recipes,
            completed_shot_count,
            total_shot_count,
        })
    }

    pub async fn update_video_segment_plan_draft_recipe_validated(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError> {
        let context = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                "更新片段规划草稿执行方案",
            )
            .await?;
        update_video_segment_plan_draft_recipe_checkpoint(
            self,
            context.run.id,
            &input.source_shot_id,
            input.segment_index_in_shot,
            &context.style,
            input.recipe,
        )
        .await
    }

    async fn video_segment_plan_draft_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        workflow_run_id: i64,
        operation: &str,
    ) -> Result<VideoSegmentPlanDraftContext, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let run = self.workflow_run_summary(workflow_run_id).await?;
        if run.workflow_kind != "video_segment_plan"
            || run.project_id != Some(project_id)
            || run.chapter_id != Some(chapter_id)
            || run.mode.as_deref() != Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(AppError::InvalidArgument(format!(
                "片段规划 workflow {workflow_run_id} 与当前项目或章节不一致"
            )));
        }
        if !workflow_run_is_running_or_resumable(&run.status, run.can_resume) {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: operation.to_string(),
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
                != Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
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
        Ok(VideoSegmentPlanDraftContext { run, style })
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
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "至少需要更新一个视频提示词字段".to_string(),
            ));
        }
        if input.duration_secs.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "视频提示词 duration_secs 必须是正整数".to_string(),
            ));
        }
        if input
            .visual_prompt
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AppError::InvalidArgument(
                "视频提示词 visual_prompt 不能为空".to_string(),
            ));
        }
        let negative_prompt = input
            .negative_prompt
            .as_ref()
            .map(|value| value.as_deref())
            .unwrap_or(prompt.negative_prompt.as_deref());
        a4n_db::video_prompt::sync_chapter_video_prompt_metadata_after_update(
            &prompt.prompt_type,
            prompt.metadata_json.clone(),
            input
                .visual_prompt
                .as_deref()
                .unwrap_or(prompt.visual_prompt.as_str()),
            negative_prompt,
            input.duration_secs.unwrap_or(prompt.duration_secs),
            prompt.reference_image_ids_json.as_ref(),
        )
        .map_err(video_db_error)?;
        let (source_result, plan_result) = futures::join!(
            self.project_chapter_video_source(
                prompt.project_id,
                prompt.chapter_id,
                "更新视频提示词",
                true,
            ),
            self.prepare_mutation_plan(
                prompt_id,
                input.downstream_strategy,
                "更新视频提示词",
                "该视频提示词",
            ),
        );
        source_result?;
        let plan = plan_result?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        let row = self
            .update_chapter_video_prompt_with_policy(
                prompt_id,
                ChapterVideoPromptPatch {
                    visual_prompt: input.visual_prompt,
                    motion_prompt: input.motion_prompt,
                    negative_prompt: input.negative_prompt,
                    duration_secs: input.duration_secs,
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
        let current_source_hash = self
            .current_video_source_hash(project_id, chapter_id)
            .await?;
        let prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?
        .ok_or_else(|| AppError::InvalidArgument("当前章节没有可用的共享片段规划".to_string()))?;
        self.chapter_video_prompt(prompt_id).await
    }

    pub(in crate::video) async fn current_shared_segment_plan_optional(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<chapter_video_prompt::Model>, AppError> {
        let current_source_hash = self
            .current_video_source_hash(project_id, chapter_id)
            .await?;
        let prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?;
        match prompt_id {
            Some(prompt_id) => self.chapter_video_prompt(prompt_id).await.map(Some),
            None => Ok(None),
        }
    }

    pub(in crate::video) async fn require_current_shared_segment_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let (prompt_result, current_source_hash_result) = futures::join!(
            self.chapter_video_prompt(prompt_id),
            self.current_video_source_hash(project_id, chapter_id),
        );
        let prompt = prompt_result?;
        self.validate_loaded_current_shared_segment_plan(
            &prompt,
            project_id,
            chapter_id,
            current_source_hash_result,
        )
        .await?;
        Ok(prompt)
    }

    async fn validate_loaded_current_shared_segment_plan(
        &self,
        prompt: &chapter_video_prompt::Model,
        project_id: i64,
        chapter_id: i64,
        current_source_hash_result: Result<String, AppError>,
    ) -> Result<(), AppError> {
        if prompt.project_id != project_id
            || prompt.chapter_id != chapter_id
            || prompt.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
            || prompt.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
            || prompt.status != chapter_video_prompt::STATUS_COMPLETED
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
        let current_source_hash = current_source_hash_result?;
        if video_prompt_source_content_hash(prompt.metadata_json.as_ref())
            != Some(current_source_hash.as_str())
        {
            return Err(AppError::InvalidArgument(
                "目标片段规划对应的章节正文已变化，请重新生成片段规划".to_string(),
            ));
        }
        let current_prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?;
        if current_prompt_id != Some(prompt.id) {
            return Err(AppError::InvalidArgument(
                "目标片段规划不是当前规划，请刷新后重试".to_string(),
            ));
        }
        Ok(())
    }

    async fn current_video_source_hash(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<String, AppError> {
        Ok(source_content_sha256(
            &self
                .project_chapter_video_source(project_id, chapter_id, "读取当前共享片段规划", false)
                .await?
                .source
                .content,
        ))
    }

    pub async fn update_chapter_video_segment_recipe_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        let (project_id, chapter_id) =
            a4n_db::video::chapter_video_prompt_scope_by_id(self.database.connection(), prompt_id)
                .await
                .map_err(video_db_error)?
                .ok_or_else(|| AppError::NotFound("视频提示词不存在".to_string()))?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let mut reference_ids = recipe.reference_image_ids.clone();
        reference_ids.extend(recipe.video_library_image_ids.iter().copied());
        reference_ids.extend(recipe.guidance_reference_image_ids.iter().copied());
        reference_ids.sort_unstable();
        reference_ids.dedup();
        self.verify_video_reference_image_access_json(
            user_id,
            project_id,
            chapter_id,
            Some(&json!(reference_ids)),
        )
        .await?;
        self.update_chapter_video_segment_recipe(prompt_id, recipe)
            .await
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
        let (source_result, plan_result) = futures::join!(
            self.project_chapter_video_source(
                prompt.project_id,
                prompt.chapter_id,
                "删除视频提示词",
                true,
            ),
            self.prepare_mutation_plan(
                prompt_id,
                downstream_strategy,
                "删除视频提示词",
                "该视频提示词",
            ),
        );
        source_result?;
        let plan = plan_result?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        self.delete_chapter_video_prompts_checked(
            std::slice::from_ref(&prompt_id),
            plan.allow_downstream,
        )
        .await
        .map(|_| ())
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
        runtime_video_config: Option<&VideoProviderConfig>,
    ) -> Result<VideoProviderConfig, AppError> {
        match runtime_video_config {
            Some(config) => Ok(config.clone()),
            None => self
                .default_video_config()
                .await
                .map(|config| super::video_provider_config_from_model(&config)),
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
            workflow_kind: "video_frame_prompts",
            workflow_step_key: "frame_prompts",
        },
        PromptGenerationStage::Segment => GenerationSpec {
            operation: "生成片段规划",
            empty_content_message: "章节正文为空，无法生成片段规划",
            workflow_kind: "video_segment_plan",
            workflow_step_key: "segment_plan",
        },
        PromptGenerationStage::Video => GenerationSpec {
            operation: "生成视频提示词",
            empty_content_message: "章节正文为空，无法生成视频提示词",
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
        chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN => "正在准备章节正文、首帧和素材",
        chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO => "正在准备章节正文和 Chat 上传素材",
        chapter_video_prompt::WORKFLOW_MODE_STORYBOARD => "正在准备章节正文、分镜图和素材",
        _ => "正在准备章节正文、边界帧和素材",
    }
}

fn ensure_video_workflow_mode(input: &VideoPromptGenerationOptions) -> Result<String, AppError> {
    let mode = input
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("视频提示词生成缺少 workflow_mode".to_string()))?;
    if !chapter_video_prompt::WORKFLOW_MODES.contains(&mode) {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词生成 workflow_mode 无效: {mode}"
        )));
    }
    Ok(mode.to_string())
}

fn ensure_video_workflow_mode_for_stage(
    input: &VideoPromptGenerationOptions,
    stage: PromptGenerationStage,
) -> Result<String, AppError> {
    let mode = ensure_video_workflow_mode(input)?;
    match stage {
        PromptGenerationStage::Segment
            if mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN =>
        {
            Err(AppError::InvalidArgument(
                "共享片段规划 workflow_mode 必须为 shared_segment_plan".to_string(),
            ))
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN =>
        {
            Err(AppError::InvalidArgument(
                "片段卡片生成必须使用卡片执行模式，不能使用 shared_segment_plan".to_string(),
            ))
        }
        _ => Ok(mode),
    }
}

fn validate_video_prompt_generation_options_for_stage(
    input: &VideoPromptGenerationOptions,
    stage: PromptGenerationStage,
) -> Result<(), AppError> {
    let workflow_mode = ensure_video_workflow_mode_for_stage(input, stage)?;
    match stage {
        PromptGenerationStage::Segment
            if input.segment_plan_prompt_id.is_some() || input.target_segment.is_some() =>
        {
            return Err(AppError::InvalidArgument(
                "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
            ));
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if input.segment_plan_prompt_id.is_none() || input.target_segment.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "片段卡片生成必须提供 segment_plan_prompt_id 和目标片段".to_string(),
            ));
        }
        _ => {}
    }
    validate_generate_prompt_options(input.clone(), &workflow_mode, None).map(|_| ())
}

pub(crate) fn validate_video_segment_plan_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Segment)
}

pub(crate) fn validate_video_frame_prompt_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Frame)
}

pub(crate) fn validate_video_final_prompt_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Video)
}

fn validate_generate_prompt_options(
    input: VideoPromptGenerationOptions,
    workflow_mode: &str,
    blank_style_placeholder: Option<&str>,
) -> Result<ChapterVideoPromptOptions, AppError> {
    let mut style = input.style.trim().to_string();
    if style.is_empty() {
        style = blank_style_placeholder
            .ok_or_else(|| AppError::InvalidArgument("视频提示词生成缺少 style".to_string()))?
            .to_string();
    }
    let prompt_output_language = PromptOutputLanguage::from_code(&input.prompt_output_language)?;
    let reference_limit = if workflow_mode == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO {
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
            target_shot_number: None,
            target_narrative_unit_index: input.target_narrative_unit_index,
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
    config: &VideoProviderConfig,
    options: &mut ChapterVideoPromptOptions,
) -> Result<(), AppError> {
    validate_video_duration_for_provider_config_secs(config, options.duration_secs)
        .map_err(AppError::InvalidArgument)?;
    let capability =
        duration_capability_for_provider_config(config).map_err(AppError::InvalidArgument)?;
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

#[async_trait]
impl a4n_api_contract::video::VideoPromptReadApplicationPort for OrchestratorContext {
    async fn list_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.list_chapter_video_prompts_validated(user_id, project_id, chapter_id)
            .await
    }

    async fn validate_project_chapter_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<(), AppError> {
        self.project_chapter_video_source(project_id, chapter_id, operation, writable)
            .await
            .map(|_| ())
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
        created_at: prompt.created_at.to_string(),
        updated_at: prompt.updated_at.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn segment_plan_options(workflow_mode: &str) -> VideoPromptGenerationOptions {
        VideoPromptGenerationOptions {
            workflow_mode: Some(workflow_mode.to_string()),
            segment_plan_prompt_id: None,
            style: "cinematic".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            prompt_batch_size: 1,
            target_segment: None,
            target_narrative_unit_index: None,
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
                chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            ))
            .is_ok()
        );
        let error = validate_video_segment_plan_generation_options(&segment_plan_options(
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
        ))
        .unwrap_err();
        assert!(error.to_string().contains("必须为 shared_segment_plan"));
    }

    #[test]
    fn stage_validation_accepts_stable_card_target_and_rejects_it_for_segment_plan() {
        let mut options = segment_plan_options(chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES);
        options.segment_plan_prompt_id = Some(7);
        options.target_segment = Some(a4n_api_contract::video::VideoSegmentTarget {
            source_shot_id: "shot-1".to_string(),
            segment_index_in_shot: 1,
        });

        assert!(validate_video_frame_prompt_generation_options(&options).is_ok());
        assert!(validate_video_final_prompt_generation_options(&options).is_ok());

        options.workflow_mode =
            Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN.to_string());
        options.segment_plan_prompt_id = None;
        let error = validate_video_segment_plan_generation_options(&options).unwrap_err();
        assert!(error.to_string().contains("不能指定片段规划 ID 或目标片段"));
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
}
