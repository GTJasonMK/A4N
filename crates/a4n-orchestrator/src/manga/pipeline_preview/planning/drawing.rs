use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::pipeline_json::{
    page_drawing_prompt_from_response, page_plan_page_number, page_plan_pages,
};
use super::super::super::pipeline_prompt::{
    MangaContextManifestInput, manga_page_drawing_prompt_user_message,
};
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_drawing_prompts(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    plan_json: &Value,
    extraction_summary: &Value,
    reference_assets: &Value,
) -> Result<(), AppError> {
    let pages = page_plan_pages(plan_json, "manga_page_planning")?;
    for (index, page_plan) in pages.iter().enumerate() {
        let page_number =
            page_plan_page_number(page_plan, &format!("manga_page_planning.pages[{index}]"))?;
        let key = format!("manga_page_drawing_prompt_page_{page_number}");
        let title = format!("漫画 Step 7：第 {page_number} 页绘图提示词");
        let call_index = i32::try_from(index).map_err(|_| {
            AppError::InvalidArgument(format!("漫画绘图提示词 preview 索引超出 i32 范围：{index}"))
        })?;
        let call_order = 800_i32.checked_add(call_index).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画绘图提示词 preview call_order 超出 i32 范围：index={index}"
            ))
        })?;
        let call = build_manga_preview_call(MangaPreviewCallInput {
            service: context.service,
            prompt_name: "manga_page_drawing_prompt",
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_page_drawing_prompt:page:{page_number}",
                context.chapter_id
            ),
            call_order,
            restore_from_checkpoint: context.should_restore_stage(7),
            manifest: MangaContextManifestInput {
                prompt_renderer: context.prompt_renderer,
                project_id: context.project_id,
                variables: &context.variables,
                chapter_content: &context.chapter_content,
                source_spans: &context.source_spans,
                prompt_names: &["manga_page_drawing_prompt"],
                previous_outputs: &[],
                reference_assets: Some(reference_assets),
                dynamic_user_content: manga_page_drawing_prompt_user_message(
                    page_number,
                    page_plan,
                    extraction_summary,
                )?,
            },
            temperature: 0.5,
        })
        .await?;
        match call {
            MangaPreviewCall::Restored { parsed_output, .. } => {
                let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
                page_drawing_prompt_from_response(&parsed_output, page_number, &page_context)?;
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    &key,
                    &title,
                    "manga_page_drawing_prompt",
                ));
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    &key,
                    &title,
                    "manga_page_drawing_prompt",
                    Some(format!("分页规划已可用；本次会发送第 {page_number} 页请求")),
                    *request,
                ));
            }
        }
    }

    Ok(())
}
