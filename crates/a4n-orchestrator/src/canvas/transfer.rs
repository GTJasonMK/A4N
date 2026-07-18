use a4n_core::error::AppError;

use super::{
    VideoStudioProjectExportRows, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows,
};
use crate::OrchestratorContext;
use crate::canvas::errors::{canvas_db_error, video_studio_project_import_error};

impl OrchestratorContext {
    pub async fn video_studio_project_export_rows(
        &self,
        project_id: i64,
    ) -> Result<VideoStudioProjectExportRows, AppError> {
        a4n_db::canvas::video_studio_project_export_rows(self.db.connection(), project_id)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn import_video_studio_project_record<Writer>(
        &self,
        user_id: i64,
        rows: VideoStudioProjectImportRows,
        now: chrono::NaiveDateTime,
        media_file_writer: &mut Writer,
    ) -> Result<a4n_core::models::project::novel_project::Model, AppError>
    where
        Writer: VideoStudioProjectImportMediaFileWriter,
    {
        a4n_db::canvas::import_video_studio_project_record(
            self.db.connection(),
            user_id,
            rows,
            now,
            media_file_writer,
        )
        .await
        .map_err(video_studio_project_import_error)
    }
}
