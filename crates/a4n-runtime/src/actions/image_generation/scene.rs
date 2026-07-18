mod execution;
mod scopes;

use super::{
    AppState, DEFAULT_IMAGE_GENERATION_PROGRESS, GeneratedImageSaveInput, GeneratedImageSummary,
    ImageChapterBindingAccess, ImageGenerationProgress, SceneImageRequestContext,
    build_scene_image_request, finalize_single_image_workflow, image_generation_request_trace_json,
    load_completed_image_checkpoint, merge_image_metadata,
    run_image_generation_provider_request_with_workflow, save_completed_image_checkpoint,
    save_generated_image_result, start_image_generation_workflow, to_summary,
};
pub use execution::generate_direct_image;
pub use scopes::{
    batch_image_generation_task_scope, object_profile_image_generation_task_scope,
    page_image_generation_task_scope, scene_image_generation_task_scope,
    scene_profile_image_generation_task_scope,
};
use serde_json::json;

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
