use a4n_core::error::AppError;
use a4n_db::workflow::delete_workflow_checkpoints_by_keys;

use crate::galgame::{
    GalgameChapterShootingScript, GalgamePipelinePromptContext, GalgamePipelineRunOptions,
    GalgamePipelineStartStage, OrchestratorContext,
};
use crate::shooting_script::ChapterShootingScriptSourceInput;
use crate::workflow::workflow_write_app_error;

use super::super::contracts::ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS;
use super::super::progress::ensure_galgame_pipeline_not_cancelled;

pub(super) async fn prepare_galgame_pipeline(
    service: &OrchestratorContext,
    project_id: i64,
    options: &GalgamePipelineRunOptions,
) -> Result<GalgamePipelinePromptContext, AppError> {
    service.ensure_media_workflow_v2_ready(project_id).await?;
    let mut context = service.galgame_pipeline_prompt_context(project_id).await?;
    let mut shooting_scripts = Vec::with_capacity(context.chapters.len());
    for chapter in &context.chapters {
        let resolved = service
            .require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter.source_chapter_id,
            })
            .await?;
        if resolved.source.source_snapshot_chapter_id != Some(chapter.source_snapshot_chapter_id)
            || resolved.source.chapter_number != chapter.chapter_number
        {
            return Err(AppError::InvalidState {
                current: format!(
                    "galgame_shooting_script_source_mismatch:source_chapter_id={}",
                    chapter.source_chapter_id
                ),
                operation: "装配 Galgame 结构化拍摄剧本".to_string(),
            });
        }
        shooting_scripts.push(GalgameChapterShootingScript {
            source_chapter_id: chapter.source_chapter_id,
            source_snapshot_chapter_id: chapter.source_snapshot_chapter_id,
            chapter_number: chapter.chapter_number,
            shooting_script_id: resolved.shooting_script.record.id,
            shooting_script_revision: resolved.shooting_script.record.revision,
            script: resolved.shooting_script.script,
        });
    }
    context.shooting_scripts = shooting_scripts;
    ensure_galgame_pipeline_not_cancelled(&service.database, options.workflow_run_id).await?;

    if !options.resume_from_checkpoint && options.start_from_stage.is_none() {
        delete_workflow_checkpoints_by_keys(
            service.database.connection(),
            options.workflow_run_id,
            ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS,
        )
        .await
        .map_err(workflow_write_app_error)?;
    }
    if let Some(start_stage) = options.start_from_stage {
        delete_workflow_checkpoints_by_keys(
            service.database.connection(),
            options.workflow_run_id,
            start_stage.checkpoint_keys_to_clear(),
        )
        .await
        .map_err(workflow_write_app_error)?;
    }

    service
        .report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::ShootingScriptProjection,
            1,
            GalgamePipelineStartStage::ShootingScriptProjection.message(),
        )
        .await?;
    Ok(context)
}
