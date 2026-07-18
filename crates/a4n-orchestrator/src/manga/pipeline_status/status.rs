use a4n_core::error::AppError;
use a4n_db::manga::{self as manga_db, MangaPipelineDbError, SetMangaPipelineStatusInput};

use super::super::OrchestratorContext;
use super::super::pipeline_state::pipeline_progress_from_layout;
use super::contracts::MangaPipelineStatusInput;

impl OrchestratorContext {
    pub(in crate::manga) async fn set_pipeline_status(
        &self,
        input: MangaPipelineStatusInput<'_>,
    ) -> Result<(), AppError> {
        manga_db::set_manga_pipeline_status(self.db.connection(), to_set_input(input))
            .await
            .map_err(manga_pipeline_db_error)
    }

    pub(super) async fn pipeline_status_row(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<a4n_core::models::chapter::chapter_manga_prompt::Model>, AppError> {
        manga_db::manga_pipeline_status_row(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(manga_pipeline_db_error)
    }

    pub(in crate::manga) async fn pipeline_progress(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(i32, i32), AppError> {
        let status_row = self
            .pipeline_status_row(project_id, chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound("漫画流水线状态记录不存在".to_string()))?;
        let progress = pipeline_progress_from_layout(status_row.layout.as_ref(), "漫画流水线状态")?;
        Ok((progress.current, progress.total))
    }
}

pub(super) fn to_set_input(input: MangaPipelineStatusInput<'_>) -> SetMangaPipelineStatusInput {
    SetMangaPipelineStatusInput {
        chapter_id: input.chapter_id,
        project_id: input.project_id,
        pipeline_status: input.pipeline_status.to_string(),
        stage: input.stage.to_string(),
        current: input.current,
        total: input.total,
        message: input.message.to_string(),
        task_id: input.task_id.map(str::to_string),
        allow_cancel_override: input.allow_cancel_override,
    }
}

pub(super) fn manga_pipeline_db_error(error: MangaPipelineDbError) -> AppError {
    match error {
        MangaPipelineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPipelineDbError::NotFound(message) => AppError::NotFound(message),
        MangaPipelineDbError::Cancelled => AppError::Cancelled,
        MangaPipelineDbError::Database(err) => AppError::Database(err),
        MangaPipelineDbError::Generation(err) => crate::generation::generation_error(err),
    }
}
