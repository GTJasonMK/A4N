use a4n_core::error::AppError;
use a4n_core::prompt_language::{PromptOutputLanguage, metadata_matches_prompt_output_language};

use crate::video::OrchestratorContext;
use crate::video::prompt_store::PromptWorkflowMode;

use super::queries::{ignored_prompt_ids_by_types, segment_plan_rows};

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_existing_frame_prompts_can_regenerate(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let rows = a4n_db::video::chapter_video_prompt_workflow_records(
        service.database.connection(),
        project_id,
        chapter_id,
        prompt_type,
        workflow_mode.as_str(),
        false,
    )
    .await
    .map_err(AppError::Database)?;
    for row in rows {
        if !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        ) {
            continue;
        }
        service
            .ensure_prompt_stage_mutable(&row, "重新生成")
            .await?;
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_existing_segment_plans_can_regenerate(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_prompt_ids_by_types(
        service,
        project_id,
        chapter_id,
        ignored_downstream_prompt_types,
        false,
    )
    .await?;
    let rows = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        false,
    )
    .await?;
    for row in rows {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_segment_plan_generation_can_start(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_prompt_ids_by_types(
        service,
        project_id,
        chapter_id,
        ignored_downstream_prompt_types,
        true,
    )
    .await?;
    let existing = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        true,
    )
    .await?
    .into_iter()
    .next();
    if let Some(row) = existing {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}
