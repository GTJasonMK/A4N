use a4n_domain::manga::{MangaPageSlotsV1, MangaPipelineArtifactStage, MangaPreprocessingV3};
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::json::{parse_manga_page_slots, required_array_field};
use super::completion::complete_pipeline;
use super::contracts::{MangaExtractionRun, MangaRunContext};
use super::page_slots::{MangaPageSlotsRun, run_page_slots};
use crate::manga::is_manga_visual_design_range;

pub(super) async fn run_single_stage(
    context: &MangaRunContext<'_>,
    stage: MangaPipelineArtifactStage,
) -> Result<Vec<a4n_domain::manga::MangaPagePromptRecord>, AppError> {
    context.check_cancelled()?;
    match stage {
        MangaPipelineArtifactStage::Preprocessing => Err(AppError::InvalidState {
            current: "preprocessing_requires_source_context".into(),
            operation: "请通过章节预处理步骤运行".into(),
        }),
        MangaPipelineArtifactStage::PageSlots => {
            let preprocessing = required_preprocessing(context).await?;
            let extraction_summary = manga_preprocessing_summary(&preprocessing)?;
            let slots = run_page_slots(context, &extraction_summary).await?;
            let extraction = MangaExtractionRun { extraction_summary };
            let scoped_context = context.with_stop_after(Some(
                a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots,
            ));
            return complete_pipeline(&scoped_context, extraction, slots).await;
        }
        MangaPipelineArtifactStage::PageContent => Err(AppError::InvalidState {
            current: "page_content_is_user_editable".into(),
            operation: "重新生成页面内容请运行分页步骤".into(),
        }),
        MangaPipelineArtifactStage::Storyboard
            if is_manga_visual_design_range(Some(stage), context.stop_after) =>
        {
            let slots = required_page_slots(context).await?;
            let preprocessing = required_preprocessing(context).await?;
            let extraction = MangaExtractionRun {
                extraction_summary: manga_preprocessing_summary(&preprocessing)?,
            };
            complete_pipeline(context, extraction, MangaPageSlotsRun { slots }).await
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
            let extraction_summary = if stage_requires_preprocessing_summary(stage) {
                manga_preprocessing_summary(&required_preprocessing(context).await?)?
            } else {
                json!({})
            };
            let extraction = MangaExtractionRun { extraction_summary };
            let slots = MangaPageSlotsRun { slots };
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
            return complete_pipeline(&scoped_context, extraction, slots).await;
        }
        MangaPipelineArtifactStage::ReferenceBindings => Err(AppError::InvalidArgument(
            "参考图绑定通过参考图面板管理".into(),
        )),
    }
}

fn stage_requires_preprocessing_summary(stage: MangaPipelineArtifactStage) -> bool {
    matches!(
        stage,
        MangaPipelineArtifactStage::Storyboard
            | MangaPipelineArtifactStage::ReferenceDiscovery
            | MangaPipelineArtifactStage::DrawingPrompt
            | MangaPipelineArtifactStage::Image
    )
}

pub(super) fn manga_preprocessing_summary(
    preprocessing: &MangaPreprocessingV3,
) -> Result<Value, AppError> {
    let events = required_array_field(
        &preprocessing.source_events,
        "events",
        "manga_preprocessing.source_events",
    )?;
    let items = required_array_field(&preprocessing.items, "items", "manga_preprocessing")?;
    Ok(json!({
        "source_events": preprocessing.source_events,
        "characters": preprocessing.characters,
        "events": events,
        "speech": preprocessing.speech,
        "visual_anchors": preprocessing.visual_anchors,
        "items": items,
        "comic_synthesis": preprocessing.comic_synthesis,
    }))
}

async fn required_preprocessing(
    context: &MangaRunContext<'_>,
) -> Result<MangaPreprocessingV3, AppError> {
    let payload = required_artifact(context, MangaPipelineArtifactStage::Preprocessing).await?;
    let preprocessing: MangaPreprocessingV3 = serde_json::from_value(payload)?;
    preprocessing
        .validate()
        .map_err(AppError::InvalidArgument)?;
    Ok(preprocessing)
}

async fn required_artifact(
    context: &MangaRunContext<'_>,
    stage: MangaPipelineArtifactStage,
) -> Result<Value, AppError> {
    let artifact = context
        .service
        .manga_pipeline_artifact(context.project_id, context.chapter_id, stage, None)
        .await?
        .ok_or_else(|| AppError::InvalidState {
            current: format!("manga_stage_artifact_missing:{}", stage.key()),
            operation: format!("{}依赖缺失的{}阶段产物", stage.key(), stage.key()),
        })?;
    if artifact.stale {
        return Err(AppError::InvalidState {
            current: format!("manga_stage_artifact_stale:{}", stage.key()),
            operation: format!(
                "{}依赖已过期的{}阶段产物，请先重跑该阶段",
                stage.key(),
                stage.key()
            ),
        });
    }
    Ok(artifact.payload)
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

#[cfg(test)]
mod tests {
    use a4n_domain::manga::MangaPipelineArtifactStage;

    use super::stage_requires_preprocessing_summary;

    #[test]
    fn image_stage_uses_the_same_preprocessing_inputs_as_reference_discovery() {
        assert!(stage_requires_preprocessing_summary(
            MangaPipelineArtifactStage::Image
        ));
        assert!(!stage_requires_preprocessing_summary(
            MangaPipelineArtifactStage::PageAdaptation
        ));
    }
}
