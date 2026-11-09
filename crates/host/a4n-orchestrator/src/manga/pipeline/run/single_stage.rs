use a4n_domain::manga::{MangaPageSlotsV1, MangaPipelineArtifactStage};
use a4n_error::AppError;
use serde_json::Value;

use super::super::json::parse_manga_page_slots;
use super::completion::complete_pipeline;
use super::contracts::MangaRunContext;
use super::page_slots::run_page_slots;
use crate::manga::is_manga_visual_design_range;

pub(super) async fn run_single_stage(
    context: &MangaRunContext<'_>,
    stage: MangaPipelineArtifactStage,
    extraction_summary: &Value,
) -> Result<Vec<a4n_domain::manga::MangaPagePromptRecord>, AppError> {
    context.check_cancelled()?;
    match stage {
        MangaPipelineArtifactStage::Preprocessing => Err(AppError::InvalidState {
            current: "preprocessing_requires_source_context".into(),
            operation: "请通过章节预处理步骤运行".into(),
        }),
        MangaPipelineArtifactStage::PageSlots => {
            let slots = run_page_slots(context, extraction_summary).await?;
            let scoped_context = context.with_stop_after(Some(
                a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots,
            ));
            return complete_pipeline(&scoped_context, extraction_summary, slots).await;
        }
        MangaPipelineArtifactStage::PageContent => Err(AppError::InvalidState {
            current: "page_content_is_user_editable".into(),
            operation: "重新生成页面内容请运行分页步骤".into(),
        }),
        MangaPipelineArtifactStage::Storyboard
            if is_manga_visual_design_range(Some(stage), context.stop_after) =>
        {
            let slots = required_page_slots(context).await?;
            complete_pipeline(context, extraction_summary, slots).await
        }
        MangaPipelineArtifactStage::PageAdaptation
        | MangaPipelineArtifactStage::Storyboard
        | MangaPipelineArtifactStage::ReferenceDiscovery
        | MangaPipelineArtifactStage::DrawingPrompt
        | MangaPipelineArtifactStage::Image => {
            let slots = required_page_slots(context).await?;
            let page_number = context
                .page_number
                .ok_or_else(|| AppError::InvalidArgument("页级漫画步骤必须指定页码".into()))?;
            require_page_artifact(
                context,
                MangaPipelineArtifactStage::PageContent,
                page_number,
            )
            .await?;
            if matches!(
                stage,
                MangaPipelineArtifactStage::Storyboard
                    | MangaPipelineArtifactStage::ReferenceDiscovery
                    | MangaPipelineArtifactStage::DrawingPrompt
                    | MangaPipelineArtifactStage::Image
            ) {
                require_page_artifact(
                    context,
                    MangaPipelineArtifactStage::PageAdaptation,
                    page_number,
                )
                .await?;
            }
            if matches!(
                stage,
                MangaPipelineArtifactStage::ReferenceDiscovery
                    | MangaPipelineArtifactStage::DrawingPrompt
                    | MangaPipelineArtifactStage::Image
            ) {
                require_page_artifact(context, MangaPipelineArtifactStage::Storyboard, page_number)
                    .await?;
            }
            if matches!(
                stage,
                MangaPipelineArtifactStage::DrawingPrompt | MangaPipelineArtifactStage::Image
            ) {
                require_page_artifact(
                    context,
                    MangaPipelineArtifactStage::ReferenceDiscovery,
                    page_number,
                )
                .await?;
            }
            if stage == MangaPipelineArtifactStage::Image {
                require_page_artifact(
                    context,
                    MangaPipelineArtifactStage::DrawingPrompt,
                    page_number,
                )
                .await?;
            }
            let stop_after = Some(match stage {
                MangaPipelineArtifactStage::PageAdaptation => {
                    a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation
                }
                MangaPipelineArtifactStage::Storyboard => {
                    a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard
                }
                MangaPipelineArtifactStage::ReferenceDiscovery => {
                    a4n_api_contract::manga::MangaPipelineStopAfter::ReferenceDiscovery
                }
                MangaPipelineArtifactStage::DrawingPrompt => {
                    a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt
                }
                MangaPipelineArtifactStage::Image => {
                    a4n_api_contract::manga::MangaPipelineStopAfter::Image
                }
                _ => unreachable!(),
            });
            let scoped_context = context.with_stop_after(stop_after);
            return complete_pipeline(&scoped_context, extraction_summary, slots).await;
        }
        MangaPipelineArtifactStage::ReferenceBindings => Err(AppError::InvalidArgument(
            "参考图绑定通过参考图面板管理".into(),
        )),
    }
}

async fn required_page_slots(context: &MangaRunContext<'_>) -> Result<MangaPageSlotsV1, AppError> {
    let artifact = context
        .service
        .manga_pipeline_artifact(
            context.project_id,
            context.chapter_id,
            MangaPipelineArtifactStage::PageSlots,
            None,
        )
        .await?
        .ok_or_else(|| AppError::InvalidState {
            current: "manga_stage_artifact_missing:page_slots".into(),
            operation: "页面步骤需要先生成分页和每页内容".into(),
        })?;
    if artifact.stale {
        return Err(AppError::InvalidState {
            current: "manga_stage_artifact_stale:page_slots".into(),
            operation: "分页产物已过期，请先重跑分页步骤".into(),
        });
    }
    parse_manga_page_slots(
        &artifact.payload,
        &context.adaptation,
        &context.source_spans,
        "manga_page_slots",
    )
}

async fn require_page_artifact(
    context: &MangaRunContext<'_>,
    stage: MangaPipelineArtifactStage,
    page_number: u32,
) -> Result<(), AppError> {
    let artifact = context
        .service
        .manga_pipeline_artifact(
            context.project_id,
            context.chapter_id,
            stage,
            Some(page_number),
        )
        .await?
        .ok_or_else(|| AppError::InvalidState {
            current: format!("manga_stage_artifact_missing:{}:{page_number}", stage.key()),
            operation: format!("当前步骤依赖缺失的{}阶段产物", stage.key()),
        })?;
    if artifact.stale {
        return Err(AppError::InvalidState {
            current: format!("manga_stage_artifact_stale:{}:{page_number}", stage.key()),
            operation: format!(
                "当前步骤依赖已过期的{}阶段产物，请先重跑该步骤",
                stage.key()
            ),
        });
    }
    Ok(())
}
