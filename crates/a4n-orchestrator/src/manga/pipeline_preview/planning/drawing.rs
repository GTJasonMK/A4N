use a4n_core::error::AppError;
use a4n_domain::manga::MangaPagePlanV3;
use serde_json::Value;

use super::super::super::pipeline_json::page_drawing_prompt_from_response;
use super::super::super::pipeline_prompt::manga_page_drawing_prompt_user_message;
use super::super::super::pipeline_state::manga_drawing_prompt_call_order;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_drawing_prompts(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    plan: &MangaPagePlanV3,
    extraction_summary: &Value,
    reference_assets: &Value,
) -> Result<(), AppError> {
    for (index, page_plan) in plan.pages.iter().enumerate() {
        let page_number = page_plan.page_number;
        let key = format!("manga_page_drawing_prompt_page_{page_number}");
        let title = format!("漫画 Step 7：第 {page_number} 页绘图提示词");
        let call_order = manga_drawing_prompt_call_order(index)?;
        let call = build_manga_preview_call(MangaPreviewCallInput {
            service: context.service,
            prompt_name: "manga_page_drawing_prompt",
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_page_drawing_prompt:page:{page_number}",
                context.chapter_id
            ),
            call_order,
            restore_from_checkpoint: context.resume_from_checkpoint,
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            user_content: manga_page_drawing_prompt_user_message(
                page_number,
                page_plan,
                extraction_summary,
                reference_assets,
            )?,
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
