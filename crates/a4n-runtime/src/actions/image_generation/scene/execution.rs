use super::super::workflow;
use super::super::{
    AppState, DEFAULT_IMAGE_GENERATION_PROGRESS, DirectImageGenerationInput,
    GeneratedImageSaveInput, GeneratedImageSummary, ImageChapterBindingAccess,
    ImageGenerationProgress, SceneImageRequestContext, access, build_direct_image_request,
    finalize_single_image_workflow, image_generation_request_trace_json,
    load_completed_image_checkpoint, merge_direct_image_metadata,
    merge_direct_image_source_metadata, run_image_generation_provider_request_with_workflow,
    save_completed_image_checkpoint, save_generated_image_result, start_image_generation_workflow,
    to_summary,
};
use serde_json::json;

pub async fn generate_direct_image(
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
