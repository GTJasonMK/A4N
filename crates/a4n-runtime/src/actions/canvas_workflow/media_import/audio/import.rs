use super::super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::source::CanvasAudioImportSource;
use crate::AppState;
use crate::file_utils::{persist_created_media_file, remove_created_file_on_error};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::media::media_asset;
use a4n_orchestrator::media::{
    CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
    MEDIA_STATUS_COMPLETED,
};
use a4n_orchestrator::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM};
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;

pub(in crate::actions::canvas_workflow::media_import) async fn import_canvas_audio(
    state: &AppState,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let asset = import_canvas_audio_asset(state, input).await?;
    Ok(ImportedCanvasMedia::Audio { asset })
}

async fn import_canvas_audio_asset(
    state: &AppState,
    input: ImportCanvasMediaInput,
) -> Result<media_asset::Model, String> {
    let project = state
        .orchestrator
        .project_by_id(input.project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {}", input.project_id))?;
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err("画布音频导入只支持 source_mode=freeform 的视频项目".to_string());
    }

    let source = CanvasAudioImportSource::from_input(&input)?;
    let output = canvas_audio_output_path(state, input.project_id, source.extension())?;
    let (output_path, _) = persist_created_media_file(
        &state.config.data_dir,
        &output,
        |output| source.write_to(output),
        "画布音频文件",
        "画布音频文件",
        "画布音频文件",
    )?;
    let asset = remove_created_file_on_error(&output, "画布音频文件", async {
        state
            .create_media_asset(CreateMediaAssetInput {
                project_id: input.project_id,
                root_project_id: input.project_id,
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: MEDIA_KIND_AUDIO.to_string(),
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
                file_path: Some(output_path),
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_COMPLETED.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: None,
                    prompt_artifact_id: None,
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: Some(source.file_name().to_string()),
                    imported_from_user: true,
                    request_snapshot_json: Some(json!({
                        "source": "video_canvas",
                        "canvas_import": true,
                        "media_kind": "audio",
                        "file_name": source.file_name(),
                        "mime_type": input.mime_type,
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await
            .map_err(|err| err.to_string())
    })
    .await?;
    Ok(asset)
}

fn canvas_audio_output_path(
    state: &AppState,
    project_id: i64,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id)
        .root()
        .join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建项目音频目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    Ok(dir.join(format!(
        "canvas_audio_{}_{}.{}",
        std::process::id(),
        stamp,
        extension
    )))
}
