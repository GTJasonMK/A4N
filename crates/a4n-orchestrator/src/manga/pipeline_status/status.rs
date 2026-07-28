use a4n_core::error::AppError;
use a4n_db::manga::MangaPipelineDbError;

use super::super::OrchestratorContext;
use super::super::pipeline_state::pipeline_progress_from_layout;

impl OrchestratorContext {
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

pub(crate) fn manga_pipeline_db_error(error: MangaPipelineDbError) -> AppError {
    match error {
        MangaPipelineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPipelineDbError::NotFound(message) => AppError::NotFound(message),
        MangaPipelineDbError::Cancelled => AppError::Cancelled,
        MangaPipelineDbError::Database(err) => AppError::Database(err),
        MangaPipelineDbError::Generation(err) => crate::generation::generation_error(err),
    }
}
