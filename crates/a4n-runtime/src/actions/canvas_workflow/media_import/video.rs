use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::canvas_media_asset_summary;
use crate::{
    AppState,
    actions::{
        app_error_message,
        video_generation::video_to_summary,
        video_records::{delete_video, import_canvas_video},
    },
};
use a4n_core::{error::AppError, models::ai::generated_video};
use a4n_orchestrator::canvas::{CanvasVideoImportHostInput, CanvasVideoImportHostPort};
use a4n_orchestrator::media::MEDIA_KIND_VIDEO;
use async_trait::async_trait;

pub(super) async fn import_canvas_video_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let host_port = RuntimeCanvasVideoImportHostPort { state, user_id };
    let result = state
        .orchestrator()
        .import_canvas_video_validated(
            &host_port,
            user_id,
            CanvasVideoImportHostInput {
                project_id: input.project_id,
                file_path: input.file_path,
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: MEDIA_KIND_VIDEO.to_string(),
        image: None,
        video: Some(video_to_summary(&result.video)),
        asset: canvas_media_asset_summary(result.asset),
    })
}

struct RuntimeCanvasVideoImportHostPort<'a> {
    state: &'a AppState,
    user_id: i64,
}

#[async_trait]
impl CanvasVideoImportHostPort for RuntimeCanvasVideoImportHostPort<'_> {
    async fn import_canvas_video(
        &self,
        input: CanvasVideoImportHostInput,
    ) -> Result<generated_video::Model, AppError> {
        import_canvas_video(self.state, input)
            .await
            .map_err(|operation| AppError::InvalidState {
                current: "canvas_video_import_host".to_string(),
                operation,
            })
    }

    async fn delete_canvas_video(&self, video_id: i64) -> Result<(), AppError> {
        delete_video(self.state, self.user_id, video_id)
            .await
            .map_err(|operation| AppError::InvalidState {
                current: "canvas_video_import_host".to_string(),
                operation,
            })
    }
}
