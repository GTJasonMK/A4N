use std::collections::{BTreeSet, HashMap};

use a4n_core::{
    dependency_conflict::DownstreamConflictStrategy,
    error::AppError,
    models::{
        ai::video_gen_config,
        chapter::{chapter, chapter_video_prompt},
    },
    prompt_language::PromptOutputLanguage,
    source_spans::source_text_hash as source_content_sha256,
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext, collaboration::ProjectAccessRole, workflow::WorkflowRunFinalizeOutcome,
};

use super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSource,
    ChapterVideoSourceContext, MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, MIN_VIDEO_KEYFRAME_IMAGES,
    UpdateChapterVideoPromptInput as DbUpdate, VideoPromptChunk, VideoPromptDownstreamDependencies,
    VideoPromptOptimizationInput, VideoPromptProgress, VideoPromptRequestPreview,
    duration_capability_for_config, validate_prompt_options_with_reference_limit,
    validate_video_duration_for_config_secs,
};

const VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
const VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
const VIDEO_WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
const VIDEO_WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
const PREVIEW_STYLE_PLACEHOLDER: &str = "未指定风格（请求预览占位；真实生成需要填写 style）";
const PREVIEW_CHAPTER_TITLE_PLACEHOLDER: &str = "未命名章节（请求预览占位；真实生成需要章节标题）";
const PREVIEW_CHAPTER_CONTENT_PLACEHOLDER: &str = "【请求预览占位正文】当前章节正文为空。真实生成前需要填写章节正文；此占位仅用于展示将发送给模型的请求结构。";
const WORKFLOW_MODES: &[&str] = &[
    VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_STORYBOARD,
];
const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratePromptOptionsInput {
    #[serde(default)]
    pub workflow_mode: Option<String>,
    pub style: String,
    #[serde(default)]
    pub prompt_output_language: Option<String>,
    pub prompt_batch_size: i32,
    #[serde(default)]
    pub target_shot_number: Option<i32>,
    pub duration_secs: i32,
    #[serde(default)]
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub video_library_image_ids: Vec<i64>,
    #[serde(default)]
    pub regenerate_existing: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoPromptInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub options: GeneratePromptOptionsInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPromptTaskScope {
    pub project_id: i64,
    pub total: u32,
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
    pub duration_secs: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub reference_image_ids_json: Option<Option<Value>>,
    #[serde(default)]
    pub guidance_mode: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub continuity_source_video_id: Option<Option<i64>>,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeChapterVideoPromptInput {
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChapterVideoPromptGroupResult {
    pub deleted_prompt_count: u64,
    pub deleted_image_count: u64,
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

impl OrchestratorContext {
    pub async fn video_prompt_task_scope_validated(
        &self,
        input: &GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptTaskScope, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(VideoPromptTaskScope {
            project_id: input.project_id,
            total: 100,
        })
    }

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
        mut input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        // 片段规划是所有卡片方案共享的上游，不继承当前卡片的生成模式。
        input.options.workflow_mode = Some(VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES.to_string());
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
            .prepare_video_prompt_generation(context.runtime_video_config, input, spec)
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
        spec: GenerationSpec,
    ) -> Result<PreparedGeneration, AppError> {
        self.video_prompt_task_scope_validated(&input).await?;
        let project_id = input.project_id;
        let chapter_id = input.chapter_id;
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
        let workflow_mode = ensure_video_workflow_mode(&input.options)?;
        let mut options = validate_generate_prompt_options(input.options, &workflow_mode, None)?;
        if workflow_mode != VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
            let config = self.effective_video_config(runtime_video_config).await?;
            apply_duration_capability(&config, &mut options)?;
        }
        self.verify_video_reference_image_access(
            input.user_id,
            project_id,
            chapter_id,
            &options.reference_image_ids,
        )
        .await?;
        let workflow_run = self
            .begin_video_prompt_workflow_validated(
                spec.workflow_kind,
                project_id,
                chapter_id,
                &workflow_mode,
                spec.workflow_step_key,
                &source_bundle.source,
                &options,
            )
            .await?;
        options.workflow_run_id = Some(workflow_run.id);
        let variables = build_video_prompt_vars(
            project_id,
            chapter_id,
            &source_bundle.chapter,
            &source_bundle.source,
            &options,
        )?;
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
                .map_err(|finalize_error| AppError::InvalidState {
                    current: message,
                    operation: format!("同步视频工作流状态失败: {finalize_error}"),
                })?;
                Err(error)
            }
        }
    }

    pub async fn preview_chapter_video_frame_prompt_requests_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.build_video_prompt_application_preview(
            context,
            input,
            false,
            "预览边界帧提示词",
            "Chat 生视频模式使用外部 Chat 视频模型，帧提示词预览不套用本地视频模型固定时长档位",
        )
        .await
    }

    pub async fn preview_chapter_video_prompt_requests_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.build_video_prompt_application_preview(
            context,
            input,
            true,
            "预览视频提示词",
            "Chat 生视频模式使用外部 Chat 视频模型，视频提示词预览不套用本地视频模型固定时长档位",
        )
        .await
    }

    async fn build_video_prompt_application_preview(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        video_stage: bool,
        source_operation: &str,
        chat_duration_note: &str,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.video_prompt_task_scope_validated(&input).await?;
        let source_bundle = self
            .project_chapter_video_source(
                input.project_id,
                input.chapter_id,
                source_operation,
                false,
            )
            .await?;
        let mut chapter = source_bundle.chapter;
        let mut source = source_bundle.source;
        let mut notes = preview_placeholders(&mut chapter, &mut source);
        let workflow_mode = ensure_video_workflow_mode(&input.options)?;
        append_preview_option_notes(&input.options, &mut notes);
        let mut options = validate_generate_prompt_options(
            input.options,
            &workflow_mode,
            Some(PREVIEW_STYLE_PLACEHOLDER),
        )?;
        if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
            notes.push(chat_duration_note.to_string());
        } else {
            let config = self
                .effective_video_config(context.runtime_video_config)
                .await?;
            apply_preview_duration_capability(&config, &mut options, &mut notes);
        }
        self.verify_video_reference_image_access(
            input.user_id,
            input.project_id,
            input.chapter_id,
            &options.reference_image_ids,
        )
        .await?;
        let variables = build_video_prompt_vars(
            input.project_id,
            input.chapter_id,
            &chapter,
            &source,
            &options,
        )?;
        let source_context = ChapterVideoSourceContext {
            content: &source.content,
            summary: source.summary.as_deref(),
            analysis: source.analysis.as_ref(),
        };
        let preview = if video_stage {
            self.preview_chapter_video_prompts_for_mode(
                &workflow_mode,
                input.project_id,
                input.chapter_id,
                source_context,
                context.prompt_renderer,
                &variables,
                options,
            )
            .await?
        } else {
            self.preview_chapter_frame_prompts_for_mode(
                &workflow_mode,
                input.project_id,
                input.chapter_id,
                source_context,
                context.prompt_renderer,
                &variables,
                options,
            )
            .await?
        };
        Ok(annotate_preview_notes(preview, &notes))
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
        if let Some(Some(ids)) = input.reference_image_ids_json.as_ref() {
            self.verify_video_reference_image_access_json(
                user_id,
                prompt.project_id,
                prompt.chapter_id,
                Some(ids),
            )
            .await?;
        }
        let metadata_patch = self
            .segment_guidance_metadata_patch(&prompt, &input)
            .await?;
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
                    duration_secs: input.duration_secs,
                    reference_image_ids_json: input.reference_image_ids_json,
                    metadata_patch,
                },
                plan.allow_downstream,
            )
            .await?;
        Ok(prompt_to_summary(&row))
    }

    async fn segment_guidance_metadata_patch(
        &self,
        prompt: &chapter_video_prompt::Model,
        input: &UpdateChapterVideoPromptInput,
    ) -> Result<Option<Value>, AppError> {
        if input.guidance_mode.is_none() && input.continuity_source_video_id.is_none() {
            return Ok(None);
        }
        if prompt.prompt_type != "video_prompt" {
            return Err(AppError::InvalidArgument(
                "只有片段视频提示词可以设置生成引导方式".to_string(),
            ));
        }
        let guidance_mode = input
            .guidance_mode
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(
                    "设置片段媒体引导时必须明确提供 guidance_mode".to_string(),
                )
            })?;
        if !matches!(
            guidance_mode,
            "system_frames" | "keyframes" | "none" | "continuity"
        ) {
            return Err(AppError::InvalidArgument(format!(
                "片段生成 guidance_mode 无效: {guidance_mode}"
            )));
        }
        let effective_references = input
            .reference_image_ids_json
            .as_ref()
            .and_then(|value| value.as_ref())
            .or(prompt.reference_image_ids_json.as_ref())
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_i64).collect::<Vec<_>>())
            .unwrap_or_default();
        if guidance_mode == "keyframes"
            && !(MIN_VIDEO_KEYFRAME_IMAGES..=MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES)
                .contains(&effective_references.len())
        {
            return Err(AppError::InvalidArgument(format!(
                "关键帧生成需要按顺序绑定 {}-{} 张图片，当前 {} 张",
                MIN_VIDEO_KEYFRAME_IMAGES,
                MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES,
                effective_references.len()
            )));
        }
        if guidance_mode == "none" && !effective_references.is_empty() {
            return Err(AppError::InvalidArgument(
                "纯文本生成不能同时绑定参考图片，请先清空当前片段参考图".to_string(),
            ));
        }
        let continuity_source = match input.continuity_source_video_id {
            Some(Some(video_id)) => {
                if guidance_mode != "continuity" || prompt.workflow_mode != "first_frame_chain" {
                    return Err(AppError::InvalidArgument(
                        "连续性来源只适用于首帧续接方案".to_string(),
                    ));
                }
                let video = self
                    .generated_video_by_id(video_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound(format!("视频 #{video_id} 不存在")))?;
                let source_shot = video
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("shot_number"))
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                if video.project_id != prompt.project_id
                    || video.chapter_id != Some(prompt.chapter_id)
                    || video.status != "completed"
                    || video.file_path.as_deref().is_none_or(str::is_empty)
                    || source_shot <= 0
                    || source_shot >= i64::from(prompt.shot_number)
                {
                    return Err(AppError::InvalidArgument(format!(
                        "连续性来源视频 #{video_id} 必须是当前章节更早片段的已完成视频"
                    )));
                }
                json!({
                    "video_id": video.id,
                    "shot_number": source_shot,
                    "provider_job_id": video.provider_job_id,
                    "file_path": video.file_path,
                    "frozen_at": chrono::Utc::now().to_rfc3339(),
                })
            }
            Some(None) => Value::Null,
            None if guidance_mode == "continuity" => {
                return Err(AppError::InvalidArgument(
                    "首帧续接需要明确选择一个更早片段的已完成视频".to_string(),
                ));
            }
            None => Value::Null,
        };
        Ok(Some(json!({
            "guidance_mode": guidance_mode,
            "keyframe_image_ids": if guidance_mode == "keyframes" { json!(effective_references) } else { Value::Null },
            "continuity_source": continuity_source,
            "continuity_stale": false,
            "continuity_replacement_video_id": Value::Null,
        })))
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

    pub async fn delete_chapter_video_prompt_group_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        group: &str,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<DeleteChapterVideoPromptGroupResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(project_id, chapter_id, "删除视频提示词", true)
            .await?;
        let rows = self
            .chapter_video_prompts_for_chapter_by_id(project_id, chapter_id)
            .await?;
        let target_rows = video_prompt_delete_group_rows(&rows, group)?;
        if target_rows.is_empty() {
            return Ok(DeleteChapterVideoPromptGroupResult {
                deleted_prompt_count: 0,
                deleted_image_count: 0,
            });
        }
        if let Some(row) = target_rows.iter().find(|row| row.status == "running") {
            return Err(AppError::InvalidState {
                current: "video_prompt_running".to_string(),
                operation: format!("视频提示词 #{} 正在运行，不能批量删除", row.id),
            });
        }
        let prompt_ids = target_rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let plan = self
            .prepare_group_mutation_plan(&prompt_ids, downstream_strategy, "批量删除视频提示词")
            .await?;
        let deleted_image_count = plan
            .delete_dependencies
            .as_ref()
            .map(|dependencies| dependencies.image_ids.len() as u64)
            .unwrap_or_default();
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        let deleted_prompt_count = self
            .delete_chapter_video_prompts_checked(&prompt_ids, plan.allow_downstream)
            .await?;
        Ok(DeleteChapterVideoPromptGroupResult {
            deleted_prompt_count,
            deleted_image_count,
        })
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

    async fn prepare_group_mutation_plan(
        &self,
        prompt_ids: &[i64],
        strategy: DownstreamConflictStrategy,
        operation: &str,
    ) -> Result<VideoPromptMutationPlan, AppError> {
        let target_ids = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut prompt_dependencies = BTreeSet::new();
        let mut image_dependencies = BTreeSet::new();
        let mut video_dependencies = BTreeSet::new();
        for prompt_id in prompt_ids {
            let dependencies = self.prompt_downstream_dependencies(*prompt_id).await?;
            prompt_dependencies.extend(
                dependencies
                    .prompt_ids
                    .into_iter()
                    .filter(|id| !target_ids.contains(id)),
            );
            image_dependencies.extend(dependencies.image_ids);
            video_dependencies.extend(dependencies.video_ids);
        }
        mutation_plan_from_dependencies(
            VideoPromptDownstreamDependencies {
                prompt_ids: prompt_dependencies.into_iter().collect(),
                image_ids: image_dependencies.into_iter().collect(),
                video_ids: video_dependencies.into_iter().collect(),
            },
            strategy,
            operation,
            "这些视频提示词",
        )
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
            target_shot_number: input.target_shot_number,
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

fn apply_preview_duration_capability(
    config: &video_gen_config::Model,
    options: &mut ChapterVideoPromptOptions,
    notes: &mut Vec<String>,
) {
    match duration_capability_for_config(config) {
        Ok(capability) => {
            if let Err(error) = capability.validate_duration(options.duration_secs) {
                notes.push(format!(
                    "当前默认视频配置不支持 {} 秒；预览仍展示请求结构，真实生成会失败：{}",
                    options.duration_secs, error
                ));
            }
            if let Err(error) = capability.validate_duration_options(&options.duration_secs_options)
            {
                notes.push(format!(
                    "当前默认视频配置不支持候选时长 {:?}；预览仍展示请求结构，真实生成会失败：{}",
                    options.duration_secs_options, error
                ));
            }
            options.duration_capability = Some(capability);
        }
        Err(error) => notes.push(format!(
            "当前默认视频配置时长能力无效；预览仍展示请求结构，真实生成会失败：{error}"
        )),
    }
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

fn preview_placeholders(
    chapter: &mut chapter::Model,
    source: &mut ChapterVideoSource,
) -> Vec<String> {
    let mut notes = Vec::new();
    if chapter
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        chapter.title = Some(PREVIEW_CHAPTER_TITLE_PLACEHOLDER.to_string());
        notes.push("章节标题为空，预览使用标题占位；真实生成仍需填写章节标题".to_string());
    }
    if source.content.trim().is_empty() {
        source.content = PREVIEW_CHAPTER_CONTENT_PLACEHOLDER.to_string();
        notes.push("章节正文为空，预览使用正文占位；真实生成仍需填写章节正文".to_string());
    }
    notes
}

fn append_preview_option_notes(input: &GeneratePromptOptionsInput, notes: &mut Vec<String>) {
    if input.style.trim().is_empty() {
        notes.push("视频风格 style 为空，预览使用风格占位；真实生成仍需填写 style".to_string());
    }
    if input.regenerate_existing {
        notes.push("已启用重新生成模式，预览会按跳过现有目标提示词复用的路径展示".to_string());
    }
}

fn annotate_preview_notes(
    mut preview: VideoPromptRequestPreview,
    notes: &[String],
) -> VideoPromptRequestPreview {
    if notes.is_empty() {
        return preview;
    }
    let note = notes.join("；");
    for step in &mut preview.steps {
        step.note = Some(match step.note.take() {
            Some(existing) if !existing.trim().is_empty() => format!("{note}；{existing}"),
            _ => note.clone(),
        });
    }
    preview
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

fn video_prompt_delete_group_rows(
    rows: &[chapter_video_prompt::Model],
    group: &str,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    match group {
        "video_prompts" => Ok(rows
            .iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .cloned()
            .collect()),
        "frame_prompts" => Ok(rows
            .iter()
            .filter(|row| {
                matches!(
                    row.prompt_type.as_str(),
                    "frame_prompt"
                        | "first_frame_prompt"
                        | "chat_video_frame_prompt"
                        | "storyboard_prompt"
                )
            })
            .cloned()
            .collect()),
        "all_chain_prompts" => Ok(rows.to_vec()),
        other => Err(AppError::InvalidArgument(format!(
            "未知视频提示词删除分组：{other}，支持 video_prompts、frame_prompts 或 all_chain_prompts"
        ))),
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
        progress(current, total, message)?;
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
}
