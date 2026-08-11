use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::{
    AppState,
    actions::{
        app_error_message,
        video_generation::video_to_summary,
        video_records::{ImportCanvasVideoInput, delete_video, import_canvas_video},
    },
};
use a4n_core::{error::AppError, models::ai::generated_video};
use a4n_orchestrator::canvas::{CanvasVideoImportHostInput, CanvasVideoImportHostPort};
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
            CanvasVideoImportHostInput {
                user_id,
                project_id: input.project_id,
                file_path: input.file_path,
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
                prompt: input.prompt,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: "video".to_string(),
        image: None,
        video: Some(
            serde_json::to_value(video_to_summary(&result.video))
                .map_err(|error| format!("序列化画布视频摘要失败: {error}"))?,
        ),
        asset: serde_json::to_value(result.asset)
            .map_err(|error| format!("序列化画布视频资产失败: {error}"))?,
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
        let summary = import_canvas_video(
            self.state,
            self.user_id,
            input.project_id,
            ImportCanvasVideoInput {
                file_path: input.file_path,
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
                prompt: input.prompt,
            },
        )
        .await
        .map_err(host_error)?;
        self.state
            .orchestrator()
            .generated_video_by_id(summary.id)
            .await
            .map_err(|error| host_error(error.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("画布导入视频不存在: {}", summary.id)))
    }

    async fn delete_canvas_video(&self, video_id: i64) -> Result<(), AppError> {
        delete_video(self.state, self.user_id, video_id)
            .await
            .map_err(host_error)
    }
}

fn host_error(operation: String) -> AppError {
    AppError::InvalidState {
        current: "canvas_video_import_host".to_string(),
        operation,
    }
}
