use super::*;

pub(in crate::video::prompt_generation::frame_prompts) async fn video_prompt_rows_for_workflow(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    a4n_db::video::chapter_video_prompt_workflow_records(
        service.database.connection(),
        project_id,
        chapter_id,
        "video_prompt",
        workflow_mode.as_str(),
        false,
    )
    .await
    .map_err(AppError::Database)
}

pub(in crate::video::prompt_generation::frame_prompts) async fn ignored_prompt_ids_by_types(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
    ensure_mutable: bool,
) -> Result<BTreeSet<i64>, AppError> {
    if prompt_types.is_empty() {
        return Ok(BTreeSet::new());
    }
    let rows = a4n_db::video::chapter_video_prompt_records_for_types(
        service.database.connection(),
        project_id,
        chapter_id,
        prompt_types,
    )
    .await
    .map_err(AppError::Database)?;
    if ensure_mutable {
        for row in &rows {
            service.ensure_prompt_stage_mutable(row, "删除").await?;
        }
    }
    Ok(rows.into_iter().map(|row| row.id).collect())
}

pub(in crate::video::prompt_generation::frame_prompts) async fn segment_plan_rows(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    only_plan_row: bool,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    a4n_db::video::chapter_video_prompt_segment_plan_records(
        service.database.connection(),
        project_id,
        chapter_id,
        workflow_mode.as_str(),
        prompt_type,
        only_plan_row,
    )
    .await
    .map_err(AppError::Database)
}
