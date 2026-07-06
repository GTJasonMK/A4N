mod batch;
mod generation_io;
mod source_binding;
mod video_frame_context;
mod workflow;

use crate::AppState;
use crate::actions::access;
use crate::actions::manga_source::{
    ChapterMangaSource, ensure_manga_source_can_write, resolve_chapter_manga_source_by_number,
};
use crate::actions::profile_materials::{
    ProfileMaterialStyleMetadata, object_profile_content, object_profile_image_prompt,
    profile_material_metadata, scene_profile_content, scene_profile_image_prompt,
};
use crate::actions::request_preview::{
    ModelRequestPreview, ModelRequestPreviewStep, image_provider_request_step,
    model_request_preview,
};
use a4n_core::models::ai::{generated_image, image_gen_config};
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_engine::image::ImageGenerationRequest;
use a4n_services::image::image_backend_supports_reference_images;
use a4n_services::manga::MANGA_PAGE_PROMPT_TYPE;
use a4n_services::source::source_content_sha256;
use batch::{batch_image_step_note, build_batch_image_jobs};
use futures::stream::{self, StreamExt};
use generation_io::{
    GeneratedImageSaveInput, image_generation_request_trace_json,
    run_image_generation_provider_request_with_workflow, save_generated_image_result,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use source_binding::{
    ImageChapterBindingAccess, collect_bound_reference_images,
    ensure_manga_page_panels_match_current_source, image_source_metadata,
    resolve_image_chapter_binding,
};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio_util::sync::CancellationToken;
use workflow::{
    batch_image_item_fingerprint, batch_image_item_step_key, finalize_batch_image_workflow,
    finalize_single_image_workflow, load_completed_image_checkpoint,
    save_completed_batch_image_checkpoint, save_completed_image_checkpoint,
    start_batch_image_workflow, start_image_generation_workflow,
};

pub(crate) use generation_io::{
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};

const IMAGE_CANCELLED_MESSAGE: &str = "图片生成已停止";
const IMAGE_METADATA_SCHEMA_VERSION: u64 = 1;
static DEFAULT_IMAGE_GENERATION_PROGRESS: ImageGenerationProgress<'static> =
    ImageGenerationProgress {
        reporter: None,
        cancel_token: None,
    };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImageSummary {
    pub id: i64,
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub ai_score: Option<f32>,
    pub is_favorite: bool,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageGenerationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone)]
struct DirectImageGenerationInput {
    project_id: i64,
    prompt: String,
    image_type: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    metadata_json: Option<Value>,
    workflow_run_id: Option<i64>,
    command_name: String,
    preview_workflow: String,
    preview_title: String,
    preview_step_key: String,
    preview_step_title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileImageInput {
    pub object_profile_id: i64,
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub style_template_name: Option<String>,
    #[serde(default)]
    pub style_template_version: Option<String>,
    #[serde(default)]
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectProfileImagePromptOutput {
    pub object_profile_id: i64,
    pub object_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileImageInput {
    pub scene_profile_id: i64,
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub style_template_name: Option<String>,
    #[serde(default)]
    pub style_template_version: Option<String>,
    #[serde(default)]
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SceneProfileImagePromptOutput {
    pub scene_profile_id: i64,
    pub scene_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateCanvasImageInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
    pub prompt: String,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateGalgameImageInput {
    pub project_id: i64,
    pub prompt: String,
    pub image_type: String,
    pub config_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub workflow_run_id: Option<i64>,
}

type ImageGenerationProgressCallback<'a> = dyn Fn(u32, u32, &str) + Send + Sync + 'a;

pub struct ImageGenerationProgress<'a> {
    reporter: Option<&'a ImageGenerationProgressCallback<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ImageGenerationProgress<'a> {
    pub fn new(
        reporter: Option<&'a ImageGenerationProgressCallback<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    fn check_cancelled(&self) -> Result<(), String> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(IMAGE_CANCELLED_MESSAGE.to_string());
        }
        Ok(())
    }

    fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.cloned().unwrap_or_default()
    }
}

struct SceneImageRequestContext {
    request: ImageGenerationRequest,
    config: image_gen_config::Model,
    chapter_id: Option<i64>,
    source_metadata: Option<Value>,
}

struct PageImageRequestContext {
    request: ImageGenerationRequest,
    config: image_gen_config::Model,
    chapter: ChapterMangaSource,
    page_prompt: chapter_manga_prompt::Model,
    panel_id: String,
    reference_bindings: Option<Value>,
    reference_image_count: usize,
    reference_image_mode: String,
}

type ProfileImageSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const OBJECT_PROFILE_IMAGE_SPEC: ProfileImageSpec = (
    "object",
    "object_material",
    "generate_object_profile_image",
    "object_profile_image_generation",
    "object_profile_image",
    "物体素材图 provider 请求",
    "素材图请求预览",
);

const SCENE_PROFILE_IMAGE_SPEC: ProfileImageSpec = (
    "scene",
    "scene_material",
    "generate_scene_profile_image",
    "scene_profile_image_generation",
    "scene_profile_image",
    "场景素材图 provider 请求",
    "场景素材图请求预览",
);

struct ProfileImageMaterial {
    spec: ProfileImageSpec,
    profile_id: i64,
    project_id: i64,
    name: String,
    prompt: String,
    profile_content: String,
    source_material: String,
    style_metadata: ProfileMaterialStyleMetadata,
}

pub async fn scene_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImageGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ImageGenerationTaskScope {
        project_id,
        total: 3,
    })
}

pub async fn page_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImageGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ImageGenerationTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn batch_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: &Value,
) -> Result<Option<ImageGenerationTaskScope>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let requests = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    if requests.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImageGenerationTaskScope {
        project_id,
        total: u32::try_from(requests.len()).unwrap_or(u32::MAX).max(1),
    }))
}

pub async fn object_profile_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ObjectProfileImageInput,
) -> Result<ImageGenerationTaskScope, String> {
    let material = build_object_profile_material(state, user_id, input).await?;
    Ok(ImageGenerationTaskScope {
        project_id: material.project_id,
        total: 3,
    })
}

pub async fn scene_profile_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SceneProfileImageInput,
) -> Result<ImageGenerationTaskScope, String> {
    let material = build_scene_profile_material(state, user_id, input).await?;
    Ok(ImageGenerationTaskScope {
        project_id: material.project_id,
        total: 3,
    })
}

pub async fn generate_scene_image(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    scene_image_generation_task_scope(state, user_id, project_id).await?;
    let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
    progress.progress(0, 3, "正在准备图片生成配置");
    progress.check_cancelled()?;
    let request_context = build_scene_image_request(
        state,
        project_id,
        prompt.clone(),
        config_id,
        chapter_id,
        ImageChapterBindingAccess::Writable,
    )
    .await?;
    let SceneImageRequestContext {
        request,
        config,
        chapter_id,
        source_metadata,
    } = request_context;
    let request_json = image_generation_request_trace_json(&config, &request);
    let metadata_json = merge_image_metadata(
        Some(json!({
            "config_id": config.id,
            "backend": config.backend,
            "image_type": "scene",
        })),
        source_metadata,
    );
    let workflow = start_image_generation_workflow(
        state,
        project_id,
        chapter_id,
        "generate_scene_image",
        "scene",
        None,
        &request_json,
        metadata_json.as_ref(),
    )
    .await?;
    let result = async {
        if let Some(summary) = load_completed_image_checkpoint(
            state,
            workflow.workflow_run_id,
            &workflow.step_key,
            &workflow.fingerprint_json,
            project_id,
        )
        .await?
        {
            progress.progress(3, 3, "图片已从断点恢复");
            return Ok(summary);
        }
        progress.progress(1, 3, "正在调用图片模型");
        progress.check_cancelled()?;
        let result = run_image_generation_provider_request_with_workflow(
            state,
            project_id,
            &config,
            request,
            Some(workflow.workflow_run_id),
            progress,
        )
        .await?;
        progress.check_cancelled()?;

        progress.progress(2, 3, "正在保存图片");
        let now = chrono::Utc::now().naive_utc();
        let img = save_generated_image_result(
            state,
            result,
            GeneratedImageSaveInput {
                project_id,
                image_type: "scene".to_string(),
                command: "generate_scene_image",
                config: &config,
                file_extra: None,
                file_context: "图片文件",
                prompt,
                request_json,
                width: config.default_width,
                height: config.default_height,
                chapter_id,
                panel_id: None,
                metadata_json,
                created_at: now,
                workflow_run_id: Some(workflow.workflow_run_id),
            },
        )
        .await?;
        let summary = to_summary(&img);
        save_completed_image_checkpoint(state, &workflow, &img, &summary).await?;
        progress.progress(3, 3, "图片已保存");
        Ok(summary)
    }
    .await;
    if let Err(status_err) =
        finalize_single_image_workflow(state, workflow.workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "场景图片生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

async fn generate_direct_image(
    state: &AppState,
    user_id: i64,
    input: DirectImageGenerationInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
    progress.progress(0, 3, "正在准备图片生成配置");
    progress.check_cancelled()?;
    let context =
        build_direct_image_request(state, &input, ImageChapterBindingAccess::Writable).await?;
    let SceneImageRequestContext {
        request,
        config,
        chapter_id,
        source_metadata,
    } = context;
    let project_id = input.project_id;
    let command_name = input.command_name;
    let image_type = input.image_type;
    let prompt = input.prompt;
    let panel_id = input.panel_id;
    let input_metadata_json = input.metadata_json;
    let external_workflow_run_id = input.workflow_run_id;
    let request_json = image_generation_request_trace_json(&config, &request);
    let base_metadata = Some(json!({
        "config_id": config.id,
        "backend": config.backend,
        "image_type": image_type.clone(),
    }));
    let metadata_json = merge_direct_image_metadata(base_metadata, input_metadata_json);
    let metadata_json = merge_direct_image_source_metadata(metadata_json, source_metadata);
    let workflow = if let Some(workflow_run_id) = external_workflow_run_id {
        workflow::attach_image_generation_workflow(
            workflow_run_id,
            project_id,
            chapter_id,
            &command_name,
            &image_type,
            panel_id.as_deref(),
            &request_json,
            metadata_json.as_ref(),
        )?
    } else {
        start_image_generation_workflow(
            state,
            project_id,
            chapter_id,
            &command_name,
            &image_type,
            panel_id.as_deref(),
            &request_json,
            metadata_json.as_ref(),
        )
        .await?
    };
    let result = async {
        if let Some(summary) = load_completed_image_checkpoint(
            state,
            workflow.workflow_run_id,
            &workflow.step_key,
            &workflow.fingerprint_json,
            project_id,
        )
        .await?
        {
            progress.progress(3, 3, "图片已从断点恢复");
            return Ok(summary);
        }
        progress.progress(1, 3, "正在调用图片模型");
        progress.check_cancelled()?;
        let result = run_image_generation_provider_request_with_workflow(
            state,
            project_id,
            &config,
            request,
            Some(workflow.workflow_run_id),
            progress,
        )
        .await?;
        progress.check_cancelled()?;

        progress.progress(2, 3, "正在保存图片");
        let now = chrono::Utc::now().naive_utc();
        let img = save_generated_image_result(
            state,
            result,
            GeneratedImageSaveInput {
                project_id,
                image_type,
                command: &command_name,
                config: &config,
                file_extra: panel_id.clone(),
                file_context: "图片文件",
                prompt,
                request_json,
                width: config.default_width,
                height: config.default_height,
                chapter_id,
                panel_id,
                metadata_json,
                created_at: now,
                workflow_run_id: Some(workflow.workflow_run_id),
            },
        )
        .await?;
        let summary = to_summary(&img);
        save_completed_image_checkpoint(state, &workflow, &img, &summary).await?;
        progress.progress(3, 3, "图片已保存");
        Ok(summary)
    }
    .await;
    if external_workflow_run_id.is_none()
        && let Err(status_err) =
            finalize_single_image_workflow(state, workflow.workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!("图片生成完成，但工作流状态写入失败: {status_err}")),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn generate_page_image(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    page_image_generation_task_scope(state, user_id, project_id).await?;
    let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
    progress.progress(0, 4, "正在读取整页漫画提示词");
    progress.check_cancelled()?;
    let request_context =
        build_page_image_request(state, project_id, chapter_number, page_number).await?;
    let PageImageRequestContext {
        request,
        config,
        chapter,
        page_prompt,
        panel_id,
        reference_bindings,
        reference_image_count,
        reference_image_mode,
    } = request_context;
    let prompt = request.prompt.clone();
    let chapter_id = Some(chapter.chapter_id);
    let page_prompt_id = page_prompt.id;
    let request_json = image_generation_request_trace_json(&config, &request);
    let metadata = json!({
        "config_id": config.id,
        "backend": config.backend,
        "chapter_number": chapter_number,
        "page_number": page_number,
        "panel_id": panel_id.clone(),
        "prompt_type": MANGA_PAGE_PROMPT_TYPE,
        "page_prompt_id": page_prompt.id,
        "generation_mode": "direct_page",
        "reference_bindings": reference_bindings,
        "reference_image_count": reference_image_count,
        "reference_image_mode": reference_image_mode,
        "source": image_source_metadata(&chapter.content),
    });
    let workflow = start_image_generation_workflow(
        state,
        project_id,
        chapter_id,
        "generate_page_image",
        "manga_page",
        Some(panel_id.as_str()),
        &request_json,
        Some(&metadata),
    )
    .await?;
    let result = async {
        if let Some(summary) = load_completed_image_checkpoint(
            state,
            workflow.workflow_run_id,
            &workflow.step_key,
            &workflow.fingerprint_json,
            project_id,
        )
        .await?
        {
            let now = chrono::Utc::now().naive_utc();
            mark_manga_page_prompt_image_completed(state, page_prompt_id, summary.id, now).await?;
            delete_replaced_manga_page_images(
                state,
                project_id,
                chapter.chapter_id,
                page_number,
                summary.id,
            )
            .await?;
            state
                .asset_lineage_sync
                .sync_project_asset_lineage(project_id)
                .await
                .map_err(|e| e.to_string())?;
            progress.progress(4, 4, "整页漫画已从断点恢复");
            return Ok(summary);
        }

        progress.progress(1, 4, "正在调用图片模型");
        progress.check_cancelled()?;
        let result = run_image_generation_provider_request_with_workflow(
            state,
            project_id,
            &config,
            request,
            Some(workflow.workflow_run_id),
            progress,
        )
        .await?;
        progress.check_cancelled()?;

        progress.progress(2, 4, "正在保存漫画页图片");
        let extra = format!("ch{chapter_number:02}_pg{page_number:02}");
        progress.progress(3, 4, "正在写入图片记录");
        let now = chrono::Utc::now().naive_utc();
        let img = save_generated_image_result(
            state,
            result,
            GeneratedImageSaveInput {
                project_id,
                image_type: "manga_page".to_string(),
                command: "generate_page_image",
                config: &config,
                file_extra: Some(extra),
                file_context: "漫画页图片",
                prompt,
                request_json,
                width: config.default_width,
                height: config.default_height,
                chapter_id,
                panel_id: Some(panel_id),
                metadata_json: Some(metadata),
                created_at: now,
                workflow_run_id: Some(workflow.workflow_run_id),
            },
        )
        .await?;
        let summary = to_summary(&img);
        save_completed_image_checkpoint(state, &workflow, &img, &summary).await?;
        mark_manga_page_prompt_image_completed(state, page_prompt_id, img.id, now).await?;
        delete_replaced_manga_page_images(
            state,
            project_id,
            chapter.chapter_id,
            page_number,
            img.id,
        )
        .await?;
        state
            .asset_lineage_sync
            .sync_project_asset_lineage(project_id)
            .await
            .map_err(|e| e.to_string())?;
        progress.progress(4, 4, "整页漫画已生成");
        Ok(summary)
    }
    .await;
    if let Err(status_err) =
        finalize_single_image_workflow(state, workflow.workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "整页漫画图片生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

async fn delete_replaced_manga_page_images(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    preserved_image_id: i64,
) -> Result<(), String> {
    let old_paths = state
        .image
        .delete_manga_page_images(
            project_id,
            chapter_id,
            page_number,
            Some(preserved_image_id),
        )
        .await
        .map_err(|e| e.to_string())?;
    for path in old_paths {
        crate::file_utils::remove_data_file_in_data_dir(
            &state.config.data_dir,
            &path,
            "旧漫画页图片",
        )?;
    }
    Ok(())
}

async fn mark_manga_page_prompt_image_completed(
    state: &AppState,
    page_prompt_id: i64,
    image_id: i64,
    updated_at: chrono::NaiveDateTime,
) -> Result<(), String> {
    let mut active_prompt = chapter_manga_prompt::ActiveModel {
        id: Set(page_prompt_id),
        ..Default::default()
    };
    active_prompt.generated_image_id = Set(Some(image_id));
    active_prompt.status = Set("completed".to_string());
    active_prompt.updated_at = Set(updated_at);
    active_prompt
        .update(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn batch_generate_images(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: Value,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<Vec<GeneratedImageSummary>, String> {
    if batch_image_generation_task_scope(state, user_id, project_id, &requests_json)
        .await?
        .is_none()
    {
        return Ok(vec![]);
    }
    let arr = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    let workflow_run = start_batch_image_workflow(state, project_id, &requests_json).await?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let total = u32::try_from(arr.len()).unwrap_or(u32::MAX).max(1);
        let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
        progress.progress(0, total, "正在准备批量图片请求");
        let jobs = build_batch_image_jobs(state, project_id, arr, true).await?;

        let concurrency = jobs
            .iter()
            .map(|job| usize::try_from(job.config.max_concurrent.max(1)).unwrap_or(1))
            .min()
            .unwrap_or(1)
            .clamp(1, jobs.len().max(1));
        progress.progress(0, total, &format!("开始批量生成图片，并发 {concurrency}"));
        let completed = Arc::new(AtomicU32::new(0));
        let mut results = stream::iter(jobs)
            .map(|job| {
                let completed = Arc::clone(&completed);
                async move {
                    let display_index = job.index + 1;
                    progress.check_cancelled()?;
                    progress.progress(
                        completed.load(Ordering::Relaxed),
                        total,
                        &format!("正在生成图片 {display_index}/{}", arr.len()),
                    );
                    let request_json =
                        image_generation_request_trace_json(&job.config, &job.request);
                    let item_fingerprint =
                        batch_image_item_fingerprint(project_id, &job, &request_json);
                    let step_key = batch_image_item_step_key(job.index);
                    if let Some(summary) = load_completed_image_checkpoint(
                        state,
                        workflow_run_id,
                        &step_key,
                        &item_fingerprint,
                        project_id,
                    )
                    .await?
                    {
                        let next_current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        progress.progress(
                            next_current,
                            total,
                            &format!("已从断点恢复图片 {display_index}/{}", arr.len()),
                        );
                        return Ok::<_, String>((job.index, summary));
                    }
                    let req = job.request.clone();
                    let result = run_image_generation_provider_request_with_workflow(
                        state,
                        project_id,
                        &job.config,
                        req,
                        Some(workflow_run_id),
                        progress,
                    )
                    .await?;
                    progress.check_cancelled()?;
                    let extra = job.chapter_id.map(|c| format!("ch{c:02}"));
                    let now = chrono::Utc::now().naive_utc();
                    let img = save_generated_image_result(
                        state,
                        result,
                        GeneratedImageSaveInput {
                            project_id,
                            image_type: job.image_type,
                            command: "batch_generate_images",
                            config: &job.config,
                            file_extra: extra,
                            file_context: "批量图片",
                            prompt: job.request.prompt,
                            request_json,
                            width: job.request.width,
                            height: job.request.height,
                            chapter_id: job.chapter_id,
                            panel_id: job.panel_id,
                            metadata_json: job.metadata_json,
                            created_at: now,
                            workflow_run_id: Some(workflow_run_id),
                        },
                    )
                    .await?;
                    let summary = to_summary(&img);
                    save_completed_batch_image_checkpoint(
                        state,
                        workflow_run_id,
                        step_key,
                        i32::try_from(job.index + 1).unwrap_or(i32::MAX),
                        item_fingerprint,
                        &img,
                        &summary,
                    )
                    .await?;
                    let next_current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    progress.progress(
                        next_current,
                        total,
                        &format!("已生成图片 {display_index}/{}", arr.len()),
                    );
                    Ok::<_, String>((job.index, to_summary(&img)))
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<Result<(usize, GeneratedImageSummary), String>>>()
            .await;

        let mut ordered = Vec::with_capacity(results.len());
        for item in results.drain(..) {
            ordered.push(item?);
        }
        ordered.sort_by_key(|(index, _)| *index);
        Ok(ordered.into_iter().map(|(_, image)| image).collect())
    }
    .await;

    if let Err(status_err) = finalize_batch_image_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "批量图片生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn preview_scene_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let context = build_scene_image_request(
        state,
        project_id,
        prompt,
        config_id,
        chapter_id,
        ImageChapterBindingAccess::ReadOnly,
    )
    .await?;
    Ok(model_request_preview(
        "image_scene_generation",
        "场景图模型请求预览",
        vec![image_generation_preview_step(
            "scene_image",
            "场景图 provider 请求",
            None,
            None,
            &context.config.backend,
            context.request,
            context.config.provider_options_json.as_ref(),
        )?],
    ))
}

pub async fn preview_canvas_image_request(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
) -> Result<ModelRequestPreview, String> {
    let direct = canvas_image_direct_input(input);
    preview_direct_image_request(state, user_id, direct).await
}

pub async fn generate_canvas_image(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = canvas_image_direct_input(input);
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn generate_galgame_image(
    state: &AppState,
    user_id: i64,
    input: GenerateGalgameImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    if input.generation_spec_id <= 0 || input.target_entity_id <= 0 {
        return Err("generation_spec_id 和 target_entity_id 必须是正整数".to_string());
    }
    let metadata_json = Some(json!({
        "galgame": {
            "generation_spec_id": input.generation_spec_id,
            "target_kind": input.target_kind,
            "target_entity_kind": input.target_entity_kind,
            "target_entity_id": input.target_entity_id,
            "workflow_run_id": input.workflow_run_id,
        },
        "source": "galgame_asset_generation",
    }));
    let direct = DirectImageGenerationInput {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: input.image_type,
        config_id: input.config_id,
        chapter_id: input.chapter_id,
        panel_id: Some(format!(
            "galgame_{}_{}",
            input.target_entity_kind, input.target_entity_id
        )),
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command_name: "generate_galgame_image".to_string(),
        preview_workflow: "galgame_image_generation".to_string(),
        preview_title: "Galgame 图片模型请求预览".to_string(),
        preview_step_key: "galgame_image".to_string(),
        preview_step_title: "Galgame 图片 provider 请求".to_string(),
    };
    generate_direct_image(state, user_id, direct, progress).await
}

fn canvas_image_direct_input(input: GenerateCanvasImageInput) -> DirectImageGenerationInput {
    let metadata_json = Some(json!({
        "canvas": {
            "graph_id": input.graph_id,
            "node_id": input.node_id,
            "workflow_run_id": input.workflow_run_id,
            "source_node_ids": input.source_node_ids,
        },
        "source": "video_canvas",
    }));
    DirectImageGenerationInput {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: "canvas_image".to_string(),
        config_id: input.config_id,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command_name: "generate_canvas_image".to_string(),
        preview_workflow: "canvas_image_generation".to_string(),
        preview_title: "画布图片模型请求预览".to_string(),
        preview_step_key: "canvas_image".to_string(),
        preview_step_title: "画布图片 provider 请求".to_string(),
    }
}

async fn preview_direct_image_request(
    state: &AppState,
    user_id: i64,
    input: DirectImageGenerationInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let context =
        build_direct_image_request(state, &input, ImageChapterBindingAccess::ReadOnly).await?;
    Ok(model_request_preview(
        input.preview_workflow,
        input.preview_title,
        vec![image_generation_preview_step(
            input.preview_step_key,
            input.preview_step_title,
            None,
            Some(format!("image_type: {}", input.image_type)),
            &context.config.backend,
            context.request,
            context.config.provider_options_json.as_ref(),
        )?],
    ))
}

pub async fn generate_object_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ObjectProfileImagePromptOutput, String> {
    let material = build_object_profile_material(state, user_id, &input).await?;
    Ok(ObjectProfileImagePromptOutput {
        object_profile_id: material.profile_id,
        object_name: material.name,
        prompt: material.prompt,
    })
}

pub async fn generate_object_profile_image(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = profile_material_direct_input(
        build_object_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn preview_object_profile_image_request(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ModelRequestPreview, String> {
    let direct = profile_material_direct_input(
        build_object_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    preview_direct_image_request(state, user_id, direct).await
}

pub async fn generate_scene_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: SceneProfileImageInput,
) -> Result<SceneProfileImagePromptOutput, String> {
    let material = build_scene_profile_material(state, user_id, &input).await?;
    Ok(SceneProfileImagePromptOutput {
        scene_profile_id: material.profile_id,
        scene_name: material.name,
        prompt: material.prompt,
    })
}

pub async fn generate_scene_profile_image(
    state: &AppState,
    user_id: i64,
    input: SceneProfileImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = profile_material_direct_input(
        build_scene_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn preview_page_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let context = build_page_image_request(state, project_id, chapter_number, page_number).await?;
    Ok(model_request_preview(
        "manga_page_image_generation",
        "整页漫画图片模型请求预览",
        vec![image_generation_preview_step(
            "page_image",
            format!("第 {chapter_number} 章第 {page_number} 页 provider 请求"),
            Some(MANGA_PAGE_PROMPT_TYPE.to_string()),
            Some(format!(
                "参考图数量：{}；参考图模式：{}",
                context.reference_image_count, context.reference_image_mode
            )),
            &context.config.backend,
            context.request,
            context.config.provider_options_json.as_ref(),
        )?],
    ))
}

pub async fn preview_batch_images_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: Value,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let arr = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    let jobs = build_batch_image_jobs(state, project_id, arr, false).await?;
    let mut steps = Vec::with_capacity(jobs.len());
    for job in jobs {
        steps.push(image_generation_preview_step(
            format!("batch_image_{}", job.index + 1),
            format!("批量图片 {}/{} provider 请求", job.index + 1, arr.len()),
            None,
            Some(batch_image_step_note(&job.image_type, &job.preview_notes)),
            &job.config.backend,
            job.request,
            job.config.provider_options_json.as_ref(),
        )?);
    }
    Ok(model_request_preview(
        "batch_image_generation",
        "批量图片模型请求预览",
        steps,
    ))
}

fn image_generation_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<String>,
    note: Option<String>,
    backend: &str,
    request: ImageGenerationRequest,
    provider_options_json: Option<&Value>,
) -> Result<ModelRequestPreviewStep, String> {
    image_provider_request_step(
        key,
        title,
        prompt_name,
        "ready",
        note,
        backend,
        request,
        provider_options_json,
    )
}

pub(crate) async fn resolve_image_config(
    state: &AppState,
    config_id: Option<i64>,
) -> Result<image_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_image_model_for_id(config_id) {
        return Ok(config);
    }
    if config_id.is_none() && crate::runtime_config::uses_system_default_image_template() {
        return resolve_system_default_image_template_with_user_key(state).await;
    }
    if let Some(id) = config_id
        && crate::runtime_config::current_image_snapshot_scope_present()
    {
        return Err(format!(
            "runtime_config_snapshot.image_configs 不包含图片配置 #{id}"
        ));
    }
    if crate::runtime_config::current_user_id().is_some() {
        return Err(match config_id {
            Some(id) => format!(
                "远程用户请求不能直接使用服务器图片配置 #{id}；请在用户配置页使用个人图片配置快照"
            ),
            None => "使用系统默认图片配置需要先在用户配置页填写并保存你的个人 API Key".to_string(),
        });
    }

    crate::global_ai_config::resolve_image_config(state.db.as_ref(), config_id)
        .await
        .map_err(|e| e.to_string())
}

async fn resolve_system_default_image_template_with_user_key(
    state: &AppState,
) -> Result<image_gen_config::Model, String> {
    let mut model = crate::global_ai_config::resolve_image_config(state.db.as_ref(), None)
        .await
        .map_err(|e| e.to_string())?;
    model.api_key_encrypted = crate::runtime_config::current_system_image_api_key();
    if model.api_key_encrypted.is_none() && image_backend_requires_api_key(&model.backend) {
        return Err("使用系统默认图片配置需要先在用户配置页填写并保存你的个人 API Key".to_string());
    }
    Ok(model)
}

fn image_backend_requires_api_key(backend: &str) -> bool {
    matches!(
        backend,
        "openai-compatible-image"
            | "openai-compatible-image-edit"
            | "openai-chat-image"
            | "cliproxy-image"
    )
}

pub(crate) fn normalize_image_metadata(metadata_json: Option<Value>) -> Option<Value> {
    match metadata_json {
        Some(Value::Object(mut map)) => {
            map.entry("metadata_schema_version".to_string())
                .or_insert_with(|| json!(IMAGE_METADATA_SCHEMA_VERSION));
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
            "value": value,
        })),
        None => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
        })),
    }
}

pub(crate) fn to_summary(img: &generated_image::Model) -> GeneratedImageSummary {
    GeneratedImageSummary {
        id: img.id,
        project_id: img.project_id,
        image_type: img.image_type.clone(),
        file_path: img.file_path.clone(),
        prompt: img.prompt.clone(),
        model: img.model.clone(),
        width: img.width,
        height: img.height,
        file_size: img.file_size,
        ai_score: img.ai_score,
        is_favorite: img.is_favorite,
        chapter_id: img.chapter_id,
        panel_id: img.panel_id.clone(),
        metadata_json: img.metadata_json.clone(),
        created_at: Some(img.created_at.to_string()),
    }
}

async fn build_scene_image_request(
    state: &AppState,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    binding_access: ImageChapterBindingAccess,
) -> Result<SceneImageRequestContext, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("prompt 不能为空".to_string());
    }
    let (chapter_id, source_metadata) =
        resolve_image_chapter_binding(state, project_id, chapter_id, binding_access).await?;
    let config = resolve_image_config(state, config_id).await?;
    Ok(SceneImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images: Vec::new(),
        },
        config,
        chapter_id,
        source_metadata,
    })
}

async fn build_direct_image_request(
    state: &AppState,
    input: &DirectImageGenerationInput,
    binding_access: ImageChapterBindingAccess,
) -> Result<SceneImageRequestContext, String> {
    let image_type = input.image_type.trim();
    if image_type.is_empty() {
        return Err("image_type 不能为空".to_string());
    }
    let prompt = input.prompt.trim();
    if prompt.is_empty() {
        return Err("prompt 不能为空".to_string());
    }
    let (chapter_id, source_metadata) =
        resolve_image_chapter_binding(state, input.project_id, input.chapter_id, binding_access)
            .await?;
    let config = resolve_image_config(state, input.config_id).await?;
    Ok(SceneImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images: Vec::new(),
        },
        config,
        chapter_id,
        source_metadata,
    })
}

macro_rules! profile_material_builder {
    (
        $fn_name:ident,
        $input_ty:ty,
        $id_field:ident,
        $service:ident,
        $id_error:literal,
        $prompt_fn:ident,
        $content_fn:ident,
        $spec:expr,
        [$($source_field:ident),+ $(,)?] $(,)?
    ) => {
        async fn $fn_name(
            state: &AppState,
            user_id: i64,
            input: &$input_ty,
        ) -> Result<ProfileImageMaterial, String> {
            if input.$id_field <= 0 {
                return Err($id_error.to_string());
            }
            let profile = state
                .$service
                .get(input.$id_field)
                .await
                .map_err(|err| err.to_string())?;
            access::verify_project_editor_string(state, user_id, profile.project_id).await?;
            let prompt = $prompt_fn(&profile, input.style.as_deref());
            let source_material = profile_source_material(
                &[$(profile.$source_field.as_str()),+],
                &prompt,
            );
            let profile_content = $content_fn(&profile);
            Ok(ProfileImageMaterial {
                spec: $spec,
                profile_id: profile.id,
                project_id: profile.project_id,
                name: profile.name,
                prompt,
                profile_content,
                source_material,
                style_metadata: profile_material_style_metadata(
                    input.style_template_id.clone(),
                    input.style_template_name.clone(),
                    input.style_template_version.clone(),
                    input.custom_style.clone(),
                ),
            })
        }
    };
}

profile_material_builder!(
    build_object_profile_material,
    ObjectProfileImageInput,
    object_profile_id,
    object_profile,
    "object_profile_id 必须大于 0",
    object_profile_image_prompt,
    object_profile_content,
    OBJECT_PROFILE_IMAGE_SPEC,
    [
        name,
        visual_description,
        material_and_state,
        story_usage,
        visual_usage,
        generation_usage,
        image_prompt_seed,
    ],
);

fn profile_material_direct_input(
    material: ProfileImageMaterial,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> DirectImageGenerationInput {
    let (
        profile_kind,
        image_type,
        command_name,
        preview_workflow,
        preview_step_key,
        preview_step_title,
        preview_title_suffix,
    ) = material.spec;
    let metadata = profile_image_metadata(
        profile_kind,
        material.profile_id,
        &material.name,
        &material.profile_content,
        &material.source_material,
        &material.prompt,
        &material.style_metadata,
    );
    DirectImageGenerationInput {
        project_id: material.project_id,
        prompt: material.prompt,
        image_type: image_type.to_string(),
        config_id,
        chapter_id,
        panel_id: Some(format!("{profile_kind}_profile_{}", material.profile_id)),
        metadata_json: Some(metadata),
        workflow_run_id: None,
        command_name: command_name.to_string(),
        preview_workflow: preview_workflow.to_string(),
        preview_title: format!("{} {preview_title_suffix}", material.name),
        preview_step_key: preview_step_key.to_string(),
        preview_step_title: preview_step_title.to_string(),
    }
}

fn profile_source_material(parts: &[&str], prompt: &str) -> String {
    let mut source = parts.join("\n");
    source.push('\n');
    source.push_str(prompt);
    source
}

fn profile_material_style_metadata(
    style_template_id: Option<String>,
    style_template_name: Option<String>,
    style_template_version: Option<String>,
    custom_style: Option<String>,
) -> ProfileMaterialStyleMetadata {
    ProfileMaterialStyleMetadata {
        style_template_id,
        style_template_name,
        style_template_version,
        custom_style,
    }
}

fn profile_image_metadata(
    kind: &str,
    profile_id: i64,
    profile_name: &str,
    profile_content: &str,
    source_material: &str,
    prompt: &str,
    style_metadata: &ProfileMaterialStyleMetadata,
) -> Value {
    let mut metadata = profile_material_metadata(
        kind,
        profile_id,
        profile_name,
        &source_content_sha256(profile_content),
        style_metadata,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert("source".to_string(), json!(format!("{kind}_profile")));
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(prompt));
        map.insert(
            "source_content_sha256".to_string(),
            json!(source_content_sha256(source_material)),
        );
    }
    metadata
}

profile_material_builder!(
    build_scene_profile_material,
    SceneProfileImageInput,
    scene_profile_id,
    scene_profile,
    "scene_profile_id 必须大于 0",
    scene_profile_image_prompt,
    scene_profile_content,
    SCENE_PROFILE_IMAGE_SPEC,
    [
        name,
        base_description,
        visual_description,
        spatial_layout,
        atmosphere,
        image_prompt_seed,
    ],
);

async fn build_page_image_request(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<PageImageRequestContext, String> {
    let chapter = resolve_chapter_manga_source_by_number(state, project_id, chapter_number, true)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&chapter, "生成漫画页图片").map_err(|e| e.to_string())?;
    if chapter.content.trim().is_empty() {
        return Err("章节正文为空，无法生成漫画页图片".into());
    }

    let page_prompt = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter.chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .filter(chapter_manga_prompt::Column::PanelNumber.eq(0))
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "第 {chapter_number} 章第 {page_number} 页没有整页漫画提示词，请先运行漫画流水线"
            )
        })?;
    ensure_manga_page_panels_match_current_source(
        std::slice::from_ref(&page_prompt),
        &chapter.content,
    )?;
    let panel_id = format!("page{page_number}");
    let reference_bindings = page_prompt
        .layout
        .as_ref()
        .and_then(|layout| layout.get("reference_bindings"))
        .cloned();
    let reference_images = collect_bound_reference_images(state, page_prompt.layout.as_ref())?;
    let reference_image_count = reference_images.len();

    let config = resolve_image_config(state, None).await?;
    let reference_image_mode = if reference_images.is_empty() {
        "none".to_string()
    } else if image_backend_supports_reference_images(&config.backend) {
        "image_input".to_string()
    } else {
        return Err(format!(
            "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或解除本页参考图绑定",
            config.backend
        ));
    };
    let image_prompt = page_prompt.image_prompt.trim();
    if image_prompt.is_empty() {
        return Err(format!(
            "第 {chapter_number} 章第 {page_number} 页整页漫画提示词为空，请先重新生成或补全提示词"
        ));
    }

    Ok(PageImageRequestContext {
        request: ImageGenerationRequest {
            prompt: image_prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images,
        },
        config,
        chapter,
        page_prompt,
        panel_id,
        reference_bindings,
        reference_image_count,
        reference_image_mode,
    })
}

fn merge_image_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
) -> Option<Value> {
    merge_source_metadata(metadata_json, source_metadata, false)
}

fn merge_direct_image_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
) -> Option<Value> {
    merge_source_metadata(metadata_json, source_metadata, true)
}

fn merge_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
    preserve_existing_source: bool,
) -> Option<Value> {
    let Some(source_metadata) = source_metadata else {
        return metadata_json;
    };

    match metadata_json {
        Some(Value::Object(mut map)) => {
            let key = if preserve_existing_source && map.contains_key("source") {
                "chapter_source"
            } else {
                "source"
            };
            map.insert(key.into(), source_metadata);
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "source": source_metadata,
        })),
        None => Some(json!({
            "source": source_metadata,
        })),
    }
}

fn merge_direct_image_metadata(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            base.extend(extra);
            Some(Value::Object(base))
        }
        (Some(Value::Object(mut base)), Some(extra)) => {
            base.insert("extra".to_string(), extra);
            Some(Value::Object(base))
        }
        (Some(base), Some(Value::Object(mut extra))) => {
            extra.insert("base".to_string(), base);
            Some(Value::Object(extra))
        }
        (Some(base), Some(extra)) => Some(json!({
            "base": base,
            "extra": extra,
        })),
        (Some(base), None) => Some(base),
        (None, Some(extra)) => Some(extra),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_image_inputs_reject_unknown_fields() {
        let object_err = serde_json::from_value::<ObjectProfileImageInput>(json!({
            "object_profile_id": 1,
            "style": "写实",
            "set_primary": true
        }))
        .expect_err("object profile image input must reject unknown fields");
        assert!(object_err.to_string().contains("unknown field"));

        let scene_err = serde_json::from_value::<SceneProfileImageInput>(json!({
            "scene_profile_id": 1,
            "style": "写实",
            "legacy_style_id": "old"
        }))
        .expect_err("scene profile image input must reject unknown fields");
        assert!(scene_err.to_string().contains("unknown field"));
    }

    #[test]
    fn profile_image_inputs_accept_documented_fields() {
        let object = serde_json::from_value::<ObjectProfileImageInput>(json!({
            "object_profile_id": 1,
            "chapter_id": 2,
            "style": "写实",
            "config_id": 3,
            "style_template_id": "template",
            "style_template_name": "模板",
            "style_template_version": "1",
            "custom_style": null
        }))
        .expect("object profile image input should accept documented fields");
        assert_eq!(object.object_profile_id, 1);
        assert_eq!(object.chapter_id, Some(2));

        let scene = serde_json::from_value::<SceneProfileImageInput>(json!({
            "scene_profile_id": 4,
            "chapter_id": null,
            "style": null,
            "config_id": null,
            "style_template_id": null,
            "style_template_name": null,
            "style_template_version": null,
            "custom_style": "手绘"
        }))
        .expect("scene profile image input should accept documented fields");
        assert_eq!(scene.scene_profile_id, 4);
        assert_eq!(scene.custom_style.as_deref(), Some("手绘"));
    }
}
