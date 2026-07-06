use crate::AppState;
use crate::actions::access;
use crate::actions::dependency_conflict::DownstreamConflictStrategy;
use crate::actions::request_preview::{
    ChatRequestStepInput, chat_request_from_preview_value, chat_request_step_for_state,
};
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use crate::actions::{image_records, video_records};
use a4n_core::error::AppError;
use a4n_core::models::ai::video_gen_config;
use a4n_core::models::chapter::{chapter, chapter_video_prompt};
use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_services::source::{metadata_source_content_sha256, source_content_sha256};
use a4n_services::video::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSourceContext,
    VideoPromptDownstreamDependencies, VideoPromptRequestPreview, VideoPromptRequestPreviewStep,
    VideoPromptUpdateInput, duration_capability_for_config,
    validate_prompt_options_with_reference_limit, validate_video_duration_for_config_secs,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use tokio_util::sync::CancellationToken;

mod optimization;

pub use self::optimization::{OptimizeChapterVideoPromptInput, optimize_chapter_video_prompt};

const VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
const VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
const VIDEO_WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
const VIDEO_WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
const PREVIEW_STYLE_PLACEHOLDER: &str = "未指定风格（请求预览占位；真实生成需要填写 style）";
const PREVIEW_CHAPTER_TITLE_PLACEHOLDER: &str = "未命名章节（请求预览占位；真实生成需要章节标题）";
const PREVIEW_CHAPTER_CONTENT_PLACEHOLDER: &str = "【请求预览占位正文】当前章节正文为空。真实生成前需要填写章节正文；此占位仅用于展示将发送给模型的请求结构。";
const CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES: &[&str] = &[
    VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_STORYBOARD,
];
const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;
pub type VideoPromptProgress<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
pub type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratePromptOptionsInput {
    #[serde(default)]
    pub workflow_mode: Option<String>,
    pub style: String,
    #[serde(default)]
    pub prompt_output_language: Option<String>,
    pub prompt_batch_size: i32,
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
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChapterVideoPromptGroupResult {
    pub deleted_prompt_count: u64,
    pub deleted_image_count: u64,
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

#[derive(Debug, Clone)]
pub struct ChapterVideoSource {
    pub content: String,
    pub summary: Option<String>,
    pub analysis: Option<Value>,
}

pub(crate) struct ProjectChapterVideoSource {
    chapter: chapter::Model,
    source: ChapterVideoSource,
}

struct PreparedChapterVideoPromptPreview {
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    source: ChapterVideoSource,
    vars: HashMap<String, Value>,
    prompt_options: ChapterVideoPromptOptions,
    placeholder_notes: Vec<String>,
}

struct ChapterVideoPromptGenerationSpec {
    initial_stop_message: &'static str,
    operation: &'static str,
    empty_content_message: &'static str,
    running_message: &'static str,
    workflow_operation: &'static str,
    workflow_kind: &'static str,
    workflow_step_key: &'static str,
}

struct PreparedChapterVideoPromptGeneration {
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    source: ChapterVideoSource,
    vars: HashMap<String, Value>,
    prompt_options: ChapterVideoPromptOptions,
    workflow_run_id: i64,
}

pub(super) struct VideoPromptMutationPlan {
    allow_downstream: bool,
    delete_dependencies: Option<VideoPromptDownstreamDependencies>,
}

pub async fn generate_chapter_video_frame_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_frame_prompts_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn generate_chapter_video_segment_plan(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_segment_plan_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn generate_chapter_video_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_prompts_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn generate_chapter_video_shot_analysis(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_shot_analysis_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn chapter_video_prompt_task_scope(
    state: &AppState,
    input: &GenerateChapterVideoPromptInput,
) -> Result<VideoPromptTaskScope, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    Ok(VideoPromptTaskScope {
        project_id: input.project_id,
        total: 100,
    })
}

pub async fn preview_chapter_video_frame_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let prepared = prepare_chapter_video_prompt_preview(
        state,
        input,
        "预览边界帧提示词",
        "预览帧提示词",
        "Chat 生视频模式使用外部 Chat 视频模型，帧提示词预览不套用本地视频模型固定时长档位",
    )
    .await?;
    let source_context = source_context(&prepared.source);
    let preview = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .video
            .preview_chapter_chat_video_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .video
            .preview_chapter_first_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .video
            .preview_chapter_storyboard_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else {
        state
            .video
            .preview_chapter_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    }
    .map_err(|e| e.to_string())?;
    replan_video_prompt_request_preview_for_state(
        state,
        annotate_preview_placeholder_inputs(preview, &prepared.placeholder_notes),
    )
    .await
}

pub async fn preview_chapter_video_frame_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_frame_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
}

pub async fn preview_chapter_video_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let prepared = prepare_chapter_video_prompt_preview(
        state,
        input,
        "预览视频提示词",
        "预览视频提示词",
        "Chat 生视频模式使用外部 Chat 视频模型，视频提示词预览不套用本地视频模型固定时长档位",
    )
    .await?;
    let source_context = source_context(&prepared.source);
    let preview = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .video
            .preview_chapter_chat_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .video
            .preview_chapter_first_frame_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .video
            .preview_chapter_storyboard_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else {
        state
            .video
            .preview_chapter_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state.engine.as_ref(),
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    }
    .map_err(|e| e.to_string())?;
    replan_video_prompt_request_preview_for_state(
        state,
        annotate_preview_placeholder_inputs(preview, &prepared.placeholder_notes),
    )
    .await
}

pub async fn preview_chapter_video_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
}

pub(crate) async fn ensure_prompt_matches_current_chapter(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
) -> Result<(), String> {
    let source = resolve_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "校验视频提示词",
        false,
    )
    .await?
    .source;
    let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref()) else {
        return Ok(());
    };
    let current_hash = source_content_sha256(&source.content);
    if source_hash != current_hash {
        return Err("当前提示词基于旧正文内容，请重新生成视频提示词".into());
    }
    Ok(())
}

pub async fn list_chapter_video_prompts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    validate_project_chapter_video_source(state, project_id, chapter_id, "查看视频提示词", false)
        .await?;
    let rows = state
        .video
        .list_prompts_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

pub async fn update_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频提示词不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "更新视频提示词",
        true,
    )
    .await?;
    if let Some(Some(reference_image_ids_json)) = input.reference_image_ids_json.as_ref() {
        verify_video_reference_image_project_access(
            state,
            user_id,
            prompt.project_id,
            prompt.chapter_id,
            Some(reference_image_ids_json),
        )
        .await?;
    }
    let mutation_plan = prepare_video_prompt_mutation_plan(
        state,
        prompt_id,
        input.downstream_strategy,
        "更新视频提示词",
    )
    .await?;
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    let row = state
        .video
        .update_prompt_with_downstream_policy(
            prompt_id,
            VideoPromptUpdateInput {
                visual_prompt: input.visual_prompt.as_deref(),
                motion_prompt: input.motion_prompt.as_ref().map(|value| value.as_deref()),
                negative_prompt: input.negative_prompt.as_ref().map(|value| value.as_deref()),
                duration_secs: input.duration_secs,
                reference_image_ids_json: input.reference_image_ids_json,
            },
            mutation_plan.allow_downstream,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(prompt_to_summary(&row))
}

pub async fn delete_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频提示词不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "删除视频提示词",
        true,
    )
    .await?;
    let mutation_plan =
        prepare_video_prompt_mutation_plan(state, prompt_id, downstream_strategy, "删除视频提示词")
            .await?;
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    if mutation_plan.allow_downstream {
        state
            .video
            .delete_prompt_allowing_downstream(prompt_id)
            .await
            .map_err(|e| e.to_string())
    } else {
        state
            .video
            .delete_prompt(prompt_id)
            .await
            .map_err(|e| e.to_string())
    }
}

pub async fn delete_chapter_video_prompt_group(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    group: &str,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<DeleteChapterVideoPromptGroupResult, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    validate_project_chapter_video_source(state, project_id, chapter_id, "删除视频提示词", true)
        .await?;

    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let target_rows = video_prompt_delete_group_rows(&rows, group)?;

    if target_rows.is_empty() {
        return Ok(DeleteChapterVideoPromptGroupResult {
            deleted_prompt_count: 0,
            deleted_image_count: 0,
        });
    }
    if let Some(row) = target_rows.iter().find(|row| row.status == "running") {
        return Err(format!("视频提示词 #{} 正在运行，不能批量删除", row.id));
    }

    let target_prompt_ids = target_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let mutation_plan = prepare_video_prompt_group_mutation_plan(
        state,
        &target_prompt_ids,
        downstream_strategy,
        "批量删除视频提示词",
    )
    .await?;
    let deleted_image_count = mutation_plan
        .delete_dependencies
        .as_ref()
        .map(|dependencies| dependencies.image_ids.len() as u64)
        .unwrap_or_default();
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    let deleted_prompt_count = state
        .video
        .delete_prompts_with_downstream_policy(&target_prompt_ids, mutation_plan.allow_downstream)
        .await
        .map_err(|e| e.to_string())?;
    Ok(DeleteChapterVideoPromptGroupResult {
        deleted_prompt_count,
        deleted_image_count,
    })
}

pub(super) async fn prepare_video_prompt_mutation_plan(
    state: &AppState,
    prompt_id: i64,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<VideoPromptMutationPlan, String> {
    let dependencies = state
        .video
        .prompt_downstream_dependencies(prompt_id)
        .await
        .map_err(|e| e.to_string())?;
    video_prompt_mutation_plan_from_dependencies(dependencies, strategy, operation, "该视频提示词")
}

async fn prepare_video_prompt_group_mutation_plan(
    state: &AppState,
    prompt_ids: &[i64],
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<VideoPromptMutationPlan, String> {
    let target_prompt_ids = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut merged = VideoPromptDownstreamDependencies::default();
    let mut downstream_prompt_ids = BTreeSet::<i64>::new();
    let mut downstream_image_ids = BTreeSet::<i64>::new();
    let mut downstream_video_ids = BTreeSet::<i64>::new();
    for prompt_id in prompt_ids {
        let dependencies = state
            .video
            .prompt_downstream_dependencies(*prompt_id)
            .await
            .map_err(|e| e.to_string())?;
        downstream_prompt_ids.extend(
            dependencies
                .prompt_ids
                .into_iter()
                .filter(|id| !target_prompt_ids.contains(id)),
        );
        downstream_image_ids.extend(dependencies.image_ids);
        downstream_video_ids.extend(dependencies.video_ids);
    }
    merged.prompt_ids = downstream_prompt_ids.into_iter().collect();
    merged.image_ids = downstream_image_ids.into_iter().collect();
    merged.video_ids = downstream_video_ids.into_iter().collect();

    video_prompt_mutation_plan_from_dependencies(merged, strategy, operation, "这些视频提示词")
}

fn video_prompt_mutation_plan_from_dependencies(
    dependencies: VideoPromptDownstreamDependencies,
    strategy: DownstreamConflictStrategy,
    operation: &str,
    subject: &str,
) -> Result<VideoPromptMutationPlan, String> {
    if dependencies.is_empty() {
        return Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: None,
        });
    }

    match strategy {
        DownstreamConflictStrategy::Reject => {
            let downstream_count = dependencies.prompt_ids.len()
                + dependencies.image_ids.len()
                + dependencies.video_ids.len();
            Err(format!(
                "{subject}已有 {downstream_count} 个下游派生产物，不能直接{operation}；请选择删除下游后继续，或保留下游继续"
            ))
        }
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

pub(super) async fn apply_video_prompt_downstream_deletions(
    state: &AppState,
    user_id: i64,
    plan: &VideoPromptMutationPlan,
) -> Result<(), String> {
    let Some(dependencies) = plan.delete_dependencies.as_ref() else {
        return Ok(());
    };

    for video_id in &dependencies.video_ids {
        video_records::delete_video(state, user_id, *video_id).await?;
    }
    for image_id in &dependencies.image_ids {
        image_records::delete_image(state, user_id, *image_id).await?;
    }
    if !dependencies.prompt_ids.is_empty() {
        state
            .video
            .delete_prompts(&dependencies.prompt_ids)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn is_generated_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt"
    )
}

fn video_prompt_delete_group_rows(
    rows: &[chapter_video_prompt::Model],
    group: &str,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    match group {
        "video_prompts" => Ok(rows
            .iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .cloned()
            .collect()),
        "frame_prompts" => Ok(rows
            .iter()
            .filter(|row| is_generated_frame_prompt_type(&row.prompt_type))
            .cloned()
            .collect()),
        "all_chain_prompts" => Ok(rows.to_vec()),
        other => Err(format!(
            "未知视频提示词删除分组：{other}，支持 video_prompts、frame_prompts 或 all_chain_prompts"
        )),
    }
}

pub(super) async fn validate_project_chapter_video_source(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<(), String> {
    resolve_project_chapter_video_source(state, project_id, chapter_id, operation, writable)
        .await
        .map(|_| ())
}

async fn prepare_chapter_video_prompt_preview(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    source_operation: &str,
    workflow_operation: &str,
    chat_duration_note: &str,
) -> Result<PreparedChapterVideoPromptPreview, String> {
    chapter_video_prompt_task_scope(state, &input).await?;
    let user_id = input.user_id;
    let project_id = input.project_id;
    let chapter_id = input.chapter_id;
    let source_bundle = resolve_project_chapter_video_source(
        state,
        project_id,
        chapter_id,
        source_operation,
        false,
    )
    .await?;
    let mut chapter = source_bundle.chapter;
    let mut source = source_bundle.source;
    let mut placeholder_notes = prepare_chapter_prompt_preview_inputs(&mut chapter, &mut source);
    let workflow_mode = ensure_video_workflow_mode(
        &input.options,
        CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES,
        workflow_operation,
    )?;
    append_prompt_option_preview_notes(&input.options, &mut placeholder_notes);
    let mut prompt_options =
        validate_generate_prompt_options_for_preview(input.options, &workflow_mode)?;
    if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        placeholder_notes.push(chat_duration_note.to_string());
    } else {
        apply_default_duration_capability_for_preview(
            state,
            &mut prompt_options,
            &mut placeholder_notes,
        )
        .await?;
    }
    verify_video_reference_image_project_access_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        &prompt_options.reference_image_ids,
    )
    .await?;
    let vars = build_video_prompt_vars(project_id, chapter_id, &chapter, &source, &prompt_options)?;
    Ok(PreparedChapterVideoPromptPreview {
        project_id,
        chapter_id,
        workflow_mode,
        source,
        vars,
        prompt_options,
        placeholder_notes,
    })
}

async fn prepare_chapter_video_prompt_generation(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: &mut Option<VideoPromptProgress<'_>>,
    cancel_token: &CancellationToken,
    spec: ChapterVideoPromptGenerationSpec,
) -> Result<PreparedChapterVideoPromptGeneration, String> {
    report_progress(progress, 0, 100, "正在检查章节和生成参数")?;
    check_cancelled(cancel_token, spec.initial_stop_message)?;
    chapter_video_prompt_task_scope(state, &input).await?;
    let user_id = input.user_id;
    let project_id = input.project_id;
    let chapter_id = input.chapter_id;
    let context =
        resolve_project_chapter_video_source(state, project_id, chapter_id, spec.operation, true)
            .await?;
    let chapter = context.chapter;
    let source = context.source;
    if source.content.trim().is_empty() {
        return Err(spec.empty_content_message.to_string());
    }
    ensure_no_running_video_prompts(state, project_id, chapter_id, spec.running_message).await?;
    let workflow_mode = ensure_video_workflow_mode(
        &input.options,
        CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES,
        spec.workflow_operation,
    )?;
    let mut prompt_options = validate_generate_prompt_options(input.options, &workflow_mode)?;
    if workflow_mode != VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        apply_default_duration_capability(state, &mut prompt_options).await?;
    }
    verify_video_reference_image_project_access_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        &prompt_options.reference_image_ids,
    )
    .await?;
    let workflow_fingerprint = video_prompt_workflow_fingerprint(
        spec.workflow_kind,
        project_id,
        chapter_id,
        &workflow_mode,
        &source,
        &prompt_options,
    );
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_prompt:{}:{project_id}:{chapter_id}:{workflow_mode}",
                spec.workflow_kind
            ),
            workflow_kind: spec.workflow_kind.to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(chapter_id),
            chapter_id: Some(chapter_id),
            mode: Some(workflow_mode.clone()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(spec.workflow_step_key.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    prompt_options.workflow_run_id = Some(workflow_run.id);
    let vars = build_video_prompt_vars(project_id, chapter_id, &chapter, &source, &prompt_options)?;
    Ok(PreparedChapterVideoPromptGeneration {
        project_id,
        chapter_id,
        workflow_mode,
        source,
        vars,
        prompt_options,
        workflow_run_id: workflow_run.id,
    })
}

async fn generate_chapter_video_frame_prompts_inner(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    mut progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let prepared = prepare_chapter_video_prompt_generation(
        state,
        input,
        &mut progress,
        &cancel_token,
        ChapterVideoPromptGenerationSpec {
            initial_stop_message: "边界帧提示词生成已停止",
            operation: "生成边界帧提示词",
            empty_content_message: "章节正文为空，无法生成边界帧提示词",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成边界帧提示词",
            workflow_operation: "生成帧提示词",
            workflow_kind: "video_frame_prompts",
            workflow_step_key: "frame_prompts",
        },
    )
    .await?;
    let stop_message = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        "Chat 首尾帧提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        "首帧提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        "分镜提示词生成已停止"
    } else {
        "边界帧提示词生成已停止"
    };
    report_progress(&mut progress, 10, 100, "正在准备章节正文、素材和时长档位")?;
    check_cancelled(&cancel_token, stop_message)?;
    let source_context = source_context(&prepared.source);
    let rows_result = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .video
            .generate_chapter_chat_video_frame_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .video
            .generate_chapter_first_frame_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .video
            .generate_chapter_storyboard_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else {
        state
            .video
            .generate_chapter_boundary_frame_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    };
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

async fn generate_chapter_video_segment_plan_inner(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    mut progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let prepared = prepare_chapter_video_prompt_generation(
        state,
        input,
        &mut progress,
        &cancel_token,
        ChapterVideoPromptGenerationSpec {
            initial_stop_message: "片段规划生成已停止",
            operation: "生成片段规划",
            empty_content_message: "章节正文为空，无法生成片段规划",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成片段规划",
            workflow_operation: "生成片段规划",
            workflow_kind: "video_segment_plan",
            workflow_step_key: "segment_plan",
        },
    )
    .await?;
    report_progress(&mut progress, 10, 100, "正在准备章节正文、素材和时长档位")?;
    check_cancelled(&cancel_token, "片段规划生成已停止")?;
    let rows_result = state
        .video
        .generate_chapter_segment_plan(
            ChapterVideoPromptStageInput {
                project_id: prepared.project_id,
                chapter_id: prepared.chapter_id,
                workflow_mode: &prepared.workflow_mode,
                source: source_context(&prepared.source),
                llm: state.llm.as_ref(),
                prompt_engine: state.engine.as_ref(),
                variables: &prepared.vars,
                options: prepared.prompt_options,
                cancel_token: Some(&cancel_token),
            },
            chunks,
            progress,
        )
        .await;
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

async fn generate_chapter_video_shot_analysis_inner(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    mut progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let prepared = prepare_chapter_video_prompt_generation(
        state,
        input,
        &mut progress,
        &cancel_token,
        ChapterVideoPromptGenerationSpec {
            initial_stop_message: "镜头分析生成已停止",
            operation: "生成镜头分析",
            empty_content_message: "章节正文为空，无法生成镜头分析",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成镜头分析",
            workflow_operation: "生成镜头分析",
            workflow_kind: "video_shot_analysis",
            workflow_step_key: "shot_analysis",
        },
    )
    .await?;
    report_progress(&mut progress, 10, 100, "正在准备章节正文、素材和时长档位")?;
    check_cancelled(&cancel_token, "镜头分析生成已停止")?;
    let rows_result = state
        .video
        .generate_chapter_shot_analysis(
            ChapterVideoPromptStageInput {
                project_id: prepared.project_id,
                chapter_id: prepared.chapter_id,
                workflow_mode: &prepared.workflow_mode,
                source: source_context(&prepared.source),
                llm: state.llm.as_ref(),
                prompt_engine: state.engine.as_ref(),
                variables: &prepared.vars,
                options: prepared.prompt_options,
                cancel_token: Some(&cancel_token),
            },
            chunks,
            progress,
        )
        .await;
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

async fn generate_chapter_video_prompts_inner(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    mut progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let prepared = prepare_chapter_video_prompt_generation(
        state,
        input,
        &mut progress,
        &cancel_token,
        ChapterVideoPromptGenerationSpec {
            initial_stop_message: "视频提示词生成已停止",
            operation: "生成视频提示词",
            empty_content_message: "章节正文为空，无法生成视频提示词",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成视频提示词",
            workflow_operation: "生成视频提示词",
            workflow_kind: "video_prompts",
            workflow_step_key: "video_prompts",
        },
    )
    .await?;
    let stop_message = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        "首帧串联视频提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        "Chat 视频提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        "分镜视频提示词生成已停止"
    } else {
        "视频提示词生成已停止"
    };
    let prepare_message = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        "正在准备章节正文、首帧和素材"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        "正在准备章节正文和 Chat 上传素材"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        "正在准备章节正文、分镜图和素材"
    } else {
        "正在准备章节正文、边界帧和素材"
    };
    report_progress(&mut progress, 10, 100, prepare_message)?;
    check_cancelled(&cancel_token, stop_message)?;
    let source_context = source_context(&prepared.source);
    let rows_result = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .video
            .generate_chapter_first_frame_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .video
            .generate_chapter_chat_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .video
            .generate_chapter_storyboard_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else {
        state
            .video
            .generate_chapter_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    };
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

pub(crate) async fn resolve_project_chapter_video_source(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<ProjectChapterVideoSource, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("章节不存在: chapter_id={chapter_id}"))?;
    match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(format!("{operation}失败：章节不属于文本项目 {project_id}"));
            }
            if chapter.status == "deleted" {
                return Err(format!(
                    "{operation}失败：第 {} 章已删除",
                    chapter.chapter_number
                ));
            }
            if writable && chapter.status == "generating" {
                return Err(format!(
                    "{operation}失败：第 {} 章正在生成中，请等待完成后再操作视频素材",
                    chapter.chapter_number
                ));
            }
            let source = resolve_chapter_video_source(&chapter)?;
            Ok(ProjectChapterVideoSource { chapter, source })
        }
        "video" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(format!(
                    "{operation}失败：章节不属于视频项目 {project_id} 绑定的根文本项目"
                ));
            }
            let snapshot_id = project
                .source_snapshot_id
                .ok_or_else(|| format!("视频项目缺少源文本快照: {project_id}"))?;
            let snapshot_chapter = source_snapshot_chapter::Entity::find()
                .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
                .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter_id))
                .one(state.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "{operation}失败：视频项目 {project_id} 的源快照不包含章节 {chapter_id}"
                    )
                })?;
            let mut chapter = chapter;
            chapter.title = snapshot_chapter.title.clone().or(chapter.title);
            chapter.content = snapshot_chapter.content.clone();
            chapter.summary = snapshot_chapter.summary.clone();
            chapter.analysis_data = snapshot_chapter.analysis_data_json.clone();
            chapter.status = "completed".to_string();
            Ok(ProjectChapterVideoSource {
                chapter,
                source: ChapterVideoSource {
                    content: snapshot_chapter.content,
                    summary: snapshot_chapter.summary,
                    analysis: snapshot_chapter.analysis_data_json,
                },
            })
        }
        other => Err(format!(
            "{operation}失败：视频链路不支持项目类型 {other}: project_id={project_id}"
        )),
    }
}

fn resolve_chapter_video_source(chapter: &chapter::Model) -> Result<ChapterVideoSource, String> {
    if chapter.status == "deleted" {
        return Err(format!(
            "第 {} 章已删除，无法生成或使用视频素材",
            chapter.chapter_number
        ));
    }
    Ok(ChapterVideoSource {
        content: chapter.content.clone(),
        summary: chapter.summary.clone(),
        analysis: chapter.analysis_data.clone(),
    })
}

fn source_context(source: &ChapterVideoSource) -> ChapterVideoSourceContext<'_> {
    ChapterVideoSourceContext {
        content: &source.content,
        summary: source.summary.as_deref(),
        analysis: source.analysis.as_ref(),
    }
}

fn build_video_prompt_vars(
    project_id: i64,
    chapter_id: i64,
    chapter: &chapter::Model,
    source: &ChapterVideoSource,
    prompt_options: &ChapterVideoPromptOptions,
) -> Result<HashMap<String, Value>, String> {
    let mut vars = HashMap::new();
    vars.insert("project_id".into(), json!(project_id));
    vars.insert("chapter_id".into(), json!(chapter_id));
    vars.insert("chapter_number".into(), json!(chapter.chapter_number));
    vars.insert(
        "chapter_title".into(),
        json!(required_chapter_title(
            chapter.title.as_deref(),
            chapter.chapter_number,
            "生成视频提示词",
        )?),
    );
    vars.insert("chapter_status".into(), json!(&chapter.status));
    vars.insert(
        "source_content_sha256".into(),
        json!(source_content_sha256(&source.content)),
    );
    if let Some(summary) = source.summary.as_deref().filter(|s| !s.trim().is_empty()) {
        vars.insert("chapter_summary".into(), json!(summary));
    }
    if let Some(analysis) = source.analysis.as_ref() {
        vars.insert("chapter_analysis".into(), analysis.clone());
        if let Some(compressed) = analysis
            .get("summaries")
            .and_then(|v| v.get("compressed"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            vars.insert("chapter_analysis_summary".into(), json!(compressed));
        }
    }
    vars.insert("style".into(), json!(&prompt_options.style));
    vars.insert(
        "prompt_output_language".into(),
        json!(
            prompt_options
                .prompt_output_language
                .as_deref()
                .unwrap_or("zh_cn")
        ),
    );
    vars.insert("duration_secs".into(), json!(prompt_options.duration_secs));
    vars.insert(
        "duration_secs_options".into(),
        json!(&prompt_options.duration_secs_options),
    );
    if let Some(capability) = prompt_options.duration_capability.as_ref() {
        vars.insert(
            "video_duration_capability".into(),
            capability.to_prompt_json(),
        );
    }
    vars.insert("aspect_ratio".into(), json!(&prompt_options.aspect_ratio));
    vars.insert(
        "reference_image_ids".into(),
        json!(&prompt_options.reference_image_ids),
    );
    vars.insert(
        "video_library_image_ids".into(),
        json!(&prompt_options.video_library_image_ids),
    );
    Ok(vars)
}

fn required_chapter_title<'a>(
    title: Option<&'a str>,
    chapter_number: i32,
    operation: &str,
) -> Result<&'a str, String> {
    title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法{operation}"))
}

fn validate_generate_prompt_options(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
) -> Result<ChapterVideoPromptOptions, String> {
    validate_generate_prompt_options_inner(input, workflow_mode, None)
}

fn validate_generate_prompt_options_for_preview(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
) -> Result<ChapterVideoPromptOptions, String> {
    validate_generate_prompt_options_inner(input, workflow_mode, Some(PREVIEW_STYLE_PLACEHOLDER))
}

fn video_prompt_workflow_fingerprint(
    workflow_kind: &str,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    source: &ChapterVideoSource,
    options: &ChapterVideoPromptOptions,
) -> Value {
    json!({
        "schema_version": 1,
        "workflow_kind": workflow_kind,
        "project_id": project_id,
        "chapter_id": chapter_id,
        "workflow_mode": workflow_mode,
        "source": {
            "content_sha256": source_content_sha256(&source.content),
            "content_chars": source.content.chars().count(),
            "summary": source.summary,
            "analysis": source.analysis,
        },
        "options": {
            "style": &options.style,
            "prompt_output_language": &options.prompt_output_language,
            "prompt_batch_size": options.prompt_batch_size,
            "duration_secs": options.duration_secs,
            "duration_secs_options": &options.duration_secs_options,
            "aspect_ratio": &options.aspect_ratio,
            "reference_image_ids": &options.reference_image_ids,
            "video_library_image_ids": &options.video_library_image_ids,
            "duration_capability": &options.duration_capability,
            "regenerate_existing": options.regenerate_existing,
        },
    })
}

async fn finalize_video_prompt_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<Vec<chapter_video_prompt::Model>, AppError>,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    match result {
        Ok(rows) => {
            state
                .workflow_run
                .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .map_err(|err| err.to_string())?;
            Ok(rows)
        }
        Err(err) => {
            let err_text = err.to_string();
            state
                .workflow_run
                .mark_status(
                    workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(&err_text),
                )
                .await
                .map_err(|status_err| {
                    format!("{err_text}; 同步视频工作流状态失败: {status_err}")
                })?;
            Err(err_text)
        }
    }
}

fn validate_generate_prompt_options_inner(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
    blank_style_placeholder: Option<&str>,
) -> Result<ChapterVideoPromptOptions, String> {
    let mut style = input.style.trim().to_string();
    if style.is_empty() {
        let Some(placeholder) = blank_style_placeholder else {
            return Err("视频提示词生成缺少 style".to_string());
        };
        style = placeholder.to_string();
    }
    let max_reference_ids = if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        MAX_CHAT_REFERENCE_IMAGE_IDS
    } else {
        MAX_NON_CHAT_REFERENCE_IMAGE_IDS
    };
    validate_prompt_options_with_reference_limit(
        ChapterVideoPromptOptions {
            style,
            prompt_output_language: input.prompt_output_language,
            prompt_batch_size: input.prompt_batch_size,
            duration_secs: input.duration_secs,
            duration_secs_options: input.duration_secs_options,
            aspect_ratio: input.aspect_ratio,
            reference_image_ids: input.reference_image_ids,
            video_library_image_ids: input.video_library_image_ids,
            duration_capability: None,
            regenerate_existing: input.regenerate_existing,
            workflow_run_id: None,
        },
        max_reference_ids,
    )
    .map_err(|err| err.to_string())
}

async fn verify_video_reference_image_project_access_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids: &[i64],
) -> Result<(), String> {
    if reference_image_ids.is_empty() {
        return Ok(());
    }
    let reference_image_ids_json = json!(reference_image_ids);
    verify_video_reference_image_project_access(
        state,
        user_id,
        project_id,
        chapter_id,
        Some(&reference_image_ids_json),
    )
    .await
}

async fn verify_video_reference_image_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<(), String> {
    let images = state
        .video
        .validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
        .await
        .map_err(|err| err.to_string())?;
    let mut project_ids = BTreeSet::new();
    for image in images {
        project_ids.insert(image.project_id);
    }
    for image_project_id in project_ids {
        access::verify_project_editor_string(state, user_id, image_project_id).await?;
    }
    Ok(())
}

fn ensure_video_workflow_mode(
    input: &GeneratePromptOptionsInput,
    allowed: &[&str],
    operation: &str,
) -> Result<String, String> {
    let mode = normalize_video_workflow_mode(input.workflow_mode.as_deref())?;
    if !allowed.contains(&mode.as_str()) {
        return Err(format!("{operation} 不支持当前视频生成模式：{mode}"));
    }
    Ok(mode)
}

fn normalize_video_workflow_mode(mode: Option<&str>) -> Result<String, String> {
    let Some(mode) = mode.map(str::trim).filter(|mode| !mode.is_empty()) else {
        return Err("视频提示词生成缺少 workflow_mode".to_string());
    };
    match mode {
        VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        | VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
        | VIDEO_WORKFLOW_MODE_CHAT_VIDEO
        | VIDEO_WORKFLOW_MODE_STORYBOARD => Ok(mode.to_string()),
        other => Err(format!("视频提示词生成 workflow_mode 无效: {other}")),
    }
}

async fn apply_default_duration_capability(
    state: &AppState,
    prompt_options: &mut ChapterVideoPromptOptions,
) -> Result<(), String> {
    let default_video_config = default_video_config_for_runtime(state).await?;
    if let Some(cfg) = default_video_config.as_ref() {
        validate_video_duration_for_config_secs(cfg, prompt_options.duration_secs)?;
        let capability = duration_capability_for_config(cfg)?;
        capability
            .validate_duration_options(&prompt_options.duration_secs_options)
            .map_err(|err| err.to_string())?;
        prompt_options.duration_capability = Some(capability);
    }
    Ok(())
}

async fn apply_default_duration_capability_for_preview(
    state: &AppState,
    prompt_options: &mut ChapterVideoPromptOptions,
    notes: &mut Vec<String>,
) -> Result<(), String> {
    let default_video_config = default_video_config_for_runtime(state).await?;
    if let Some(cfg) = default_video_config.as_ref() {
        apply_preview_duration_capability(cfg, prompt_options, notes);
    }
    Ok(())
}

async fn default_video_config_for_runtime(
    state: &AppState,
) -> Result<Option<video_gen_config::Model>, String> {
    if let Some(config) = crate::runtime_config::current_video_model() {
        return Ok(Some(config));
    }
    crate::global_ai_config::resolve_video_config(state.db.as_ref(), None)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

fn apply_preview_duration_capability(
    cfg: &video_gen_config::Model,
    prompt_options: &mut ChapterVideoPromptOptions,
    notes: &mut Vec<String>,
) {
    match duration_capability_for_config(cfg) {
        Ok(capability) => {
            if let Err(err) = capability.validate_duration(prompt_options.duration_secs) {
                notes.push(format!(
                    "当前默认视频配置不支持 {} 秒；预览仍展示请求结构，真实生成会失败：{}",
                    prompt_options.duration_secs, err
                ));
            }
            if let Err(err) =
                capability.validate_duration_options(&prompt_options.duration_secs_options)
            {
                notes.push(format!(
                    "当前默认视频配置不支持候选时长 {:?}；预览仍展示请求结构，真实生成会失败：{}",
                    prompt_options.duration_secs_options, err
                ));
            }
            prompt_options.duration_capability = Some(capability);
        }
        Err(err) => notes.push(format!(
            "当前默认视频配置时长能力无效；预览仍展示请求结构，真实生成会失败：{err}"
        )),
    }
}

fn prepare_chapter_prompt_preview_inputs(
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

fn annotate_preview_placeholder_inputs(
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

fn append_prompt_option_preview_notes(input: &GeneratePromptOptionsInput, notes: &mut Vec<String>) {
    if input.style.trim().is_empty() {
        notes.push("视频风格 style 为空，预览使用风格占位；真实生成仍需填写 style".to_string());
    }
    if input.regenerate_existing {
        notes.push("已启用重新生成模式，预览会按跳过现有目标提示词复用的路径展示".to_string());
    }
}

async fn video_prompt_preview_to_model_request_preview(
    state: &AppState,
    preview: VideoPromptRequestPreview,
) -> Result<ModelRequestPreview, String> {
    let mut steps = Vec::new();
    for step in preview.steps {
        let rebuilt_request = match step.request.as_ref() {
            Some(request) => Some(chat_request_from_preview_value(
                request,
                step.context_manifest.clone(),
            )?),
            None => None,
        };
        let converted = match rebuilt_request {
            Some(request) => {
                let mut converted = chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: Some(step.prompt_name),
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?;
                converted.kind = "llm_chat".to_string();
                converted
            }
            None => ModelRequestPreviewStep {
                key: step.key,
                title: step.title,
                kind: "llm_chat".to_string(),
                prompt_name: Some(step.prompt_name),
                state: step.state,
                note: step.note,
                system_prompt: step.system_prompt,
                user_prompt: step.user_prompt,
                messages: step.messages,
                request: step.request,
                context_manifest: step.context_manifest,
                context_plan: step.context_plan,
                context_pack_refs: step.context_pack_refs,
                total_chars: step.total_chars,
                char_counts: step.char_counts,
            },
        };
        steps.push(converted);
    }
    Ok(crate::actions::request_preview::model_request_preview(
        preview.workflow,
        preview.title,
        steps,
    ))
}

async fn replan_video_prompt_request_preview_for_state(
    state: &AppState,
    preview: VideoPromptRequestPreview,
) -> Result<VideoPromptRequestPreview, String> {
    let mut steps = Vec::new();
    for step in preview.steps {
        let rebuilt_request = match step.request.as_ref() {
            Some(request) => Some(chat_request_from_preview_value(
                request,
                step.context_manifest.clone(),
            )?),
            None => None,
        };
        let replanned = match rebuilt_request {
            Some(request) => {
                let converted = chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: Some(step.prompt_name),
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?;
                VideoPromptRequestPreviewStep {
                    key: converted.key,
                    title: converted.title,
                    prompt_name: converted.prompt_name.unwrap_or_default(),
                    state: converted.state,
                    note: converted.note,
                    system_prompt: converted.system_prompt,
                    user_prompt: converted.user_prompt,
                    messages: converted.messages,
                    request: converted.request,
                    context_manifest: converted.context_manifest,
                    context_plan: converted.context_plan,
                    context_pack_refs: converted.context_pack_refs,
                    total_chars: converted.total_chars,
                    char_counts: converted.char_counts,
                }
            }
            None => step,
        };
        steps.push(replanned);
    }
    Ok(VideoPromptRequestPreview {
        workflow: preview.workflow,
        title: preview.title,
        steps,
    })
}

async fn ensure_no_running_video_prompts(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    message: &str,
) -> Result<(), String> {
    let running = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::Status.eq("running"))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    if running > 0 {
        return Err(message.to_string());
    }
    Ok(())
}

fn report_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken, message: &str) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(message.to_string());
    }
    Ok(())
}

pub(super) fn prompt_to_summary(p: &chapter_video_prompt::Model) -> ChapterVideoPromptSummary {
    ChapterVideoPromptSummary {
        id: p.id,
        project_id: p.project_id,
        chapter_id: p.chapter_id,
        style: p.style.clone(),
        prompt_type: p.prompt_type.clone(),
        workflow_mode: p.workflow_mode.clone(),
        shot_number: p.shot_number,
        title: p.title.clone(),
        scene_summary: p.scene_summary.clone(),
        visual_prompt: p.visual_prompt.clone(),
        motion_prompt: p.motion_prompt.clone(),
        negative_prompt: p.negative_prompt.clone(),
        duration_secs: p.duration_secs,
        aspect_ratio: p.aspect_ratio.clone(),
        reference_image_ids_json: p.reference_image_ids_json.clone(),
        status: p.status.clone(),
        generated_video_id: p.generated_video_id,
        metadata_json: p.metadata_json.clone(),
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    }
}

#[cfg(test)]
mod tests;
