use a4n_error::AppError;

use crate::galgame::{
    GalgamePipelinePromptContext, GalgamePipelineRunOptions, GalgamePipelineStartStage,
    OrchestratorContext,
};

use super::super::contracts::ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS;
use super::super::progress::ensure_galgame_pipeline_not_cancelled;

pub(super) async fn prepare_galgame_pipeline(
    service: &OrchestratorContext,
    project_id: i64,
    options: &GalgamePipelineRunOptions,
    renderer: &dyn a4n_prompt::rendering::PromptRenderer,
) -> Result<GalgamePipelinePromptContext, AppError> {
    let mut context = service.galgame_pipeline_prompt_context(project_id).await?;
    ensure_galgame_pipeline_not_cancelled(service, options.workflow_run_id).await?;

    if !options.resume_from_checkpoint && options.start_from_stage.is_none() {
        service
            .delete_workflow_checkpoints_by_keys(
                options.workflow_run_id,
                ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS,
            )
            .await?;
    }
    if let Some(start_stage) = options.start_from_stage {
        service
            .delete_workflow_checkpoints_by_keys(
                options.workflow_run_id,
                start_stage.checkpoint_keys_to_clear(),
            )
            .await?;
    }
    if options.start_from_stage == Some(GalgamePipelineStartStage::AssetGeneration) {
        return Ok(context);
    }

    service
        .report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::AdaptationPlan,
            1,
            GalgamePipelineStartStage::AdaptationPlan.message(),
        )
        .await?;
    let ids = context
        .chapters
        .iter()
        .map(|chapter| chapter.source_chapter_id)
        .collect::<Vec<_>>();
    let sources = a4n_db::project::resolve_snapshot_sources(
        service.database.executor(),
        project_id,
        context.root_project_id,
        context.source_snapshot_id,
        &ids,
    )
    .await
    .map_err(crate::shooting_script::errors::resolve_source_error)?;
    context.adaptations.clear();
    for source in sources {
        let adaptation = service
            .ensure_adaptation_plan(
                &source,
                a4n_domain::media_authoring::AdaptationTarget::Galgame,
                options.workflow_run_id,
                "galgame_pipeline",
                renderer,
                None,
                None,
            )
            .await?;
        context
            .adaptations
            .push(crate::galgame::GalgameChapterAdaptation {
                source_chapter_id: source.source_chapter_id,
                source_snapshot_chapter_id: source
                    .source_snapshot_chapter_id
                    .ok_or_else(|| AppError::InvalidArgument("Galgame 原文快照身份缺失".into()))?,
                chapter_number: source.chapter_number,
                adaptation_plan_id: adaptation.record.id,
                adaptation_plan_revision: adaptation.record.revision,
                plan: adaptation.plan,
            });
    }
    Ok(context)
}
