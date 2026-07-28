use super::super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::source::CanvasAudioImportSource;
use crate::AppState;
use crate::{
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::persist_created_media_file,
};
use a4n_core::error::AppError;
use a4n_core::files::paths::ProjectPaths;
use a4n_orchestrator::canvas::{
    CanvasAudioImportHostInput, CanvasAudioImportHostPort, PersistedCanvasAudioFile,
};
use async_trait::async_trait;
use std::path::PathBuf;

pub(in crate::actions::canvas_workflow::media_import) async fn import_canvas_audio(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let host_port = RuntimeCanvasAudioImportHostPort { state };
    let asset = state
        .canvas()
        .import_canvas_audio_validated(
            &host_port,
            CanvasAudioImportHostInput {
                user_id,
                project_id: input.project_id,
                file_path: input.file_path,
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
            },
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: "audio".to_string(),
        image: None,
        video: None,
        asset: serde_json::to_value(asset)
            .map_err(|error| format!("序列化画布音频资产失败: {error}"))?,
    })
}

struct RuntimeCanvasAudioImportHostPort<'a> {
    state: &'a AppState,
}

#[async_trait]
impl CanvasAudioImportHostPort for RuntimeCanvasAudioImportHostPort<'_> {
    async fn persist_canvas_audio(
        &self,
        input: CanvasAudioImportHostInput,
    ) -> Result<PersistedCanvasAudioFile, AppError> {
        let runtime_input = ImportCanvasMediaInput {
            project_id: input.project_id,
            media_kind: "audio".to_string(),
            file_path: input.file_path,
            file_name: input.file_name,
            mime_type: input.mime_type.clone(),
            data_base64: input.data_base64,
            prompt: None,
        };
        let source = CanvasAudioImportSource::from_input(&runtime_input)
            .map_err(AppError::InvalidArgument)?;
        let output = canvas_audio_output_path(self.state, input.project_id, source.extension())
            .map_err(AppError::InvalidArgument)?;
        let (relative_path, _) = persist_created_media_file(
            self.state.data_dir(),
            &output,
            |file| source.write_to(file),
            "画布音频文件",
            "画布音频文件",
            "画布音频文件",
        )
        .map_err(|error| AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: error,
        })?;
        Ok(PersistedCanvasAudioFile {
            relative_path,
            source_file_name: source.file_name().to_string(),
            mime_type: input.mime_type,
        })
    }

    async fn remove_canvas_audio(&self, relative_path: &str) -> Result<(), AppError> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("canvas-audio-cleanup:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
        .map_err(|error| AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: error,
        })
    }
}

fn canvas_audio_output_path(
    state: &AppState,
    project_id: i64,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(state.data_dir(), project_id)
        .root()
        .join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建项目音频目录：{err}"))?;
    let stamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1000);
    Ok(dir.join(format!(
        "canvas_audio_{}_{}.{}",
        std::process::id(),
        stamp,
        extension
    )))
}
