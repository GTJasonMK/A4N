use crate::AppState;

use super::super::config::to_summary;
use super::super::contracts::{
    DEFAULT_IMAGE_GENERATION_PROGRESS, GeneratedImageSummary, ImageGenerationProgress,
};
use super::super::generation_io::{
    GeneratedImageSaveInput, run_image_generation_provider_request_with_workflow,
    save_generated_image_result,
};
use super::super::scene::page_image_generation_task_scope;
use super::super::workflow::{
    finalize_single_image_workflow, load_completed_image_checkpoint,
    save_completed_image_checkpoint, start_image_generation_workflow,
};
use super::completion::complete_manga_page_image;
use super::preparation::prepare_page_image_generation;

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
    let prepared =
        prepare_page_image_generation(state, project_id, chapter_number, page_number).await?;
    let super::preparation::PreparedPageImageGeneration {
        context,
        request_json,
        metadata,
    } = prepared;
    let super::super::contracts::PageImageRequestContext {
        request,
        config,
        chapter,
        page_prompt,
        panel_id,
        ..
    } = context;
    let prompt = request.prompt.clone();
    let chapter_id = Some(chapter.chapter_id);
    let page_prompt_id = page_prompt.id;
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
            complete_manga_page_image(
                state,
                project_id,
                chapter.chapter_id,
                page_number,
                page_prompt_id,
                summary.id,
                now,
            )
            .await?;
            progress.progress(4, 4, "整页漫画已从断点恢复");
            return Ok(summary);
        }

        progress.progress(1, 4, "正在调用图片模型");
        progress.check_cancelled()?;
        let provider_result = run_image_generation_provider_request_with_workflow(
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
            provider_result,
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
        complete_manga_page_image(
            state,
            project_id,
            chapter.chapter_id,
            page_number,
            page_prompt_id,
            img.id,
            now,
        )
        .await?;
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
