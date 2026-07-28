use a4n_core::error::AppError;
use a4n_db::manga as manga_db;

use super::super::OrchestratorContext;
use super::super::pipeline_state::pipeline_status_from_layout;
use super::contracts::MangaPipelineStatusInput;
use super::status::manga_pipeline_db_error;

impl OrchestratorContext {
    pub(in crate::manga) async fn ensure_pipeline_not_cancelled(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let status_row = self.pipeline_status_row(project_id, chapter_id).await?;

        let Some(status_row) = status_row else {
            return Ok(());
        };
        let pipeline_status =
            pipeline_status_from_layout(status_row.layout.as_ref(), "漫画流水线状态")?;
        if pipeline_status == "cancelled" || status_row.status == "cancelled" {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }

    pub(crate) async fn cancel_manga_pipeline_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<i64, AppError> {
        let count =
            manga_db::cancel_manga_page_prompts(self.database.connection(), project_id, chapter_id)
                .await
                .map_err(manga_pipeline_db_error)?;

        let (current, total) = self.pipeline_progress(project_id, chapter_id).await?;

        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "cancelled",
            stage: "cancelled",
            current,
            total,
            message: "已请求取消漫画流水线",
            task_id: None,
            allow_cancel_override: true,
        })
        .await?;

        Ok(count)
    }
}
