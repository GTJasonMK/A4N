use super::super::GalgameMediaBindingResult;
use super::binding::{bind_galgame_media_inner, validate_galgame_media_target};
use super::cleanup::{append_cleanup_error, cleanup_imported_media_asset};
use super::rules::{ImportedAudioSource, normalize_audio_role};
use super::{BindGalgameMediaInput, ImportGalgameAudioInput};
use crate::{
    AppState,
    actions::access,
    file_utils::{persist_created_media_file, remove_created_file_on_error},
};
use a4n_core::files::paths::ProjectPaths;
use a4n_orchestrator::galgame::GALGAME_ENTITY_LINE;
use a4n_orchestrator::media::{
    CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
    MEDIA_STATUS_COMPLETED,
};
use chrono::Utc;
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub async fn import_galgame_audio(
    state: &AppState,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let role = normalize_audio_role(&input.role)?;
    let target_context = validate_galgame_media_target(
        state,
        input.project_id,
        GALGAME_ENTITY_LINE,
        input.target_line_id,
    )
    .await?;
    let source =
        ImportedAudioSource::from_upload(&input.file_name, &input.mime_type, &input.data_base64)?;
    let (output, output_path, file_size) = persist_imported_galgame_audio(
        &state.config.data_dir,
        input.project_id,
        source.extension,
        &source.bytes,
    )?;
    let asset = remove_created_file_on_error(&output, "Galgame 音频文件", async {
        state
            .create_media_asset(CreateMediaAssetInput {
                project_id: input.project_id,
                root_project_id: target_context.root_project_id,
                source_snapshot_id: Some(target_context.source_snapshot_id),
                source_chapter_id: Some(target_context.source_chapter_id),
                source_snapshot_chapter_id: Some(target_context.source_snapshot_chapter_id),
                media_kind: MEDIA_KIND_AUDIO.to_string(),
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
                file_path: Some(output_path.clone()),
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
                    source_external_path: Some(source.file_name.clone()),
                    imported_from_user: true,
                    request_snapshot_json: Some(json!({
                        "source": "galgame_audio_import",
                        "media_kind": "audio",
                        "target_line_id": input.target_line_id,
                        "role": &role,
                        "file_name": source.file_name,
                        "mime_type": source.mime_type,
                        "file_size": file_size,
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await
            .map_err(|err| err.to_string())
    })
    .await?;
    match bind_galgame_media_inner(
        state,
        BindGalgameMediaInput {
            project_id: input.project_id,
            media_asset_id: asset.id,
            target_kind: GALGAME_ENTITY_LINE.to_string(),
            target_id: input.target_line_id,
            role,
        },
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            let cleanup_error = cleanup_imported_media_asset(state, asset.id, &output_path)
                .await
                .err();
            Err(append_cleanup_error(err, cleanup_error))
        }
    }
}

pub(super) fn persist_imported_galgame_audio(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
    bytes: &[u8],
) -> Result<(PathBuf, String, i64), String> {
    let (path, mut output_file) = create_galgame_audio_output(data_dir, project_id, extension)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(bytes)
                .map_err(|err| format!("写入 Galgame 音频文件失败：{err}"))
        },
        "Galgame 音频文件",
        "Galgame 音频文件",
        "Galgame 音频文件",
    )?;
    Ok((path, relative_path, file_size))
}

fn create_galgame_audio_output(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let dir = ProjectPaths::new(data_dir, project_id).root().join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建 Galgame 音频目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("galgame_audio_{stamp}{suffix}.{extension}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("创建 Galgame 音频文件失败：{err}")),
        }
    }
    Err("创建 Galgame 音频文件失败：文件名冲突过多".to_string())
}
