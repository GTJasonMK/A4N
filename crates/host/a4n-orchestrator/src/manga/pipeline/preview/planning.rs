mod page_pipeline;
mod page_slots;

use a4n_domain::manga::MangaPipelineArtifactStage;
use a4n_error::AppError;
use serde_json::Value;

use super::contracts::{MangaPipelineRequestPreviewStep, MangaPreviewContext};
use page_pipeline::{preview_page_pipeline, preview_single_page_stage};
use page_slots::preview_page_slots;

pub(super) async fn preview_planning(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: Value,
) -> Result<(), AppError> {
    if let Some(stage) = context.run_stage {
        match stage {
            MangaPipelineArtifactStage::PageSlots => {
                preview_page_slots(context, steps, &extraction_summary).await?;
            }
            MangaPipelineArtifactStage::PageAdaptation
            | MangaPipelineArtifactStage::Storyboard
            | MangaPipelineArtifactStage::ReferenceDiscovery
            | MangaPipelineArtifactStage::DrawingPrompt => {
                preview_single_page_stage(context, steps, &extraction_summary, stage).await?;
            }
            MangaPipelineArtifactStage::Image => {
                preview_single_page_stage(context, steps, &extraction_summary, stage).await?;
                let page_number = context
                    .page_number
                    .ok_or_else(|| AppError::InvalidArgument("成图步骤必须指定页码".into()))?;
                steps.push(MangaPipelineRequestPreviewStep {
                    key: "manga_page_image".into(),
                    title: format!("第 {page_number} 页成图"),
                    prompt_name: None,
                    state: "not_applicable".into(),
                    note: Some("成图由图像生成服务执行，不会发送聊天模型请求".into()),
                    request: None,
                });
            }
            MangaPipelineArtifactStage::Preprocessing => {
                return Err(AppError::InvalidState {
                    current: "manga_preprocessing_preview_unrouted".into(),
                    operation: "漫画预处理必须单独预览".into(),
                });
            }
            MangaPipelineArtifactStage::PageContent
            | MangaPipelineArtifactStage::ReferenceBindings => {
                return Err(AppError::InvalidArgument(
                    "该步骤由用户编辑，不生成模型请求".into(),
                ));
            }
        }
        return Ok(());
    }

    let Some(slots) = preview_page_slots(context, steps, &extraction_summary).await? else {
        return Ok(());
    };
    if matches!(
        context.stop_after,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots)
    ) {
        return Ok(());
    }
    preview_page_pipeline(context, steps, &slots, &extraction_summary).await
}
