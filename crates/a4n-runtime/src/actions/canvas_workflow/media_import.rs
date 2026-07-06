use super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;
use crate::actions::access;
use crate::actions::image_records::{UploadImageInput, upload_image};
use crate::actions::video_records::{ImportCanvasVideoInput, import_canvas_video};
use crate::file_utils::data_dir_relative_file_path;
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    media::media_asset,
    project::novel_project,
};
use a4n_services::media::{
    CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
    MEDIA_STATUS_COMPLETED,
};
use a4n_services::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM};
use base64::Engine as _;
use chrono::Utc;
use sea_orm::EntityTrait;
use serde_json::json;
use std::path::{Path, PathBuf};

const MAX_IMPORT_AUDIO_BYTES: u64 = 256 * 1024 * 1024;

pub async fn import_canvas_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    match input.media_kind.trim() {
        "image" => {
            let image = upload_image(
                state,
                user_id,
                input.project_id,
                UploadImageInput {
                    file_name: input
                        .file_name
                        .ok_or_else(|| "导入画布图片缺少 file_name".to_string())?,
                    mime_type: input
                        .mime_type
                        .ok_or_else(|| "导入画布图片缺少 mime_type".to_string())?,
                    data_base64: input
                        .data_base64
                        .ok_or_else(|| "导入画布图片缺少 data_base64".to_string())?,
                    image_type: "canvas_image".to_string(),
                    prompt: input.prompt.unwrap_or_else(|| "导入画布图片".to_string()),
                    model: Some("external-import".to_string()),
                    chapter_id: None,
                    panel_id: None,
                    metadata_json: Some(json!({
                        "canvas_import": true,
                        "source": "video_canvas"
                    })),
                },
            )
            .await?;
            let asset = sync_imported_canvas_image_asset(state, input.project_id, image.id).await?;
            Ok(ImportedCanvasMedia::Image { image, asset })
        }
        "video" => {
            let video = import_canvas_video(
                state,
                user_id,
                input.project_id,
                ImportCanvasVideoInput {
                    file_path: input.file_path,
                    file_name: input.file_name,
                    mime_type: input.mime_type,
                    data_base64: input.data_base64,
                    prompt: input.prompt,
                },
            )
            .await?;
            let asset = sync_imported_canvas_video_asset(state, input.project_id, video.id).await?;
            Ok(ImportedCanvasMedia::Video { video, asset })
        }
        "audio" => {
            let asset = import_canvas_audio_asset(state, input).await?;
            Ok(ImportedCanvasMedia::Audio { asset })
        }
        other => Err(format!("画布媒体类型无效: {other}")),
    }
}

async fn sync_imported_canvas_image_asset(
    state: &AppState,
    project_id: i64,
    image_id: i64,
) -> Result<media_asset::Model, String> {
    let image = generated_image::Entity::find_by_id(image_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布导入图片不存在: {image_id}"))?;
    super::ensure_canvas_image_media_asset(
        state.db.as_ref(),
        project_id,
        &image,
        None,
        MEDIA_ORIGIN_IMPORTED,
    )
    .await
}

async fn sync_imported_canvas_video_asset(
    state: &AppState,
    project_id: i64,
    video_id: i64,
) -> Result<media_asset::Model, String> {
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布导入视频不存在: {video_id}"))?;
    super::ensure_canvas_video_media_asset(
        state.db.as_ref(),
        project_id,
        &video,
        None,
        MEDIA_ORIGIN_IMPORTED,
    )
    .await
}

async fn import_canvas_audio_asset(
    state: &AppState,
    input: ImportCanvasMediaInput,
) -> Result<media_asset::Model, String> {
    let project = novel_project::Entity::find_by_id(input.project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("项目不存在: {}", input.project_id))?;
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err("画布音频导入只支持 source_mode=freeform 的视频项目".to_string());
    }

    let source = CanvasAudioImportSource::from_input(&input)?;
    let output = canvas_audio_output_path(state, input.project_id, source.extension)?;
    if let Some(bytes) = source.bytes.as_ref() {
        std::fs::write(&output, bytes).map_err(|err| format!("写入画布音频文件失败：{err}"))?;
    } else if let Some(path) = source.path.as_ref() {
        std::fs::copy(path, &output).map_err(|err| format!("复制画布音频文件失败：{err}"))?;
    } else {
        return Err("导入画布音频缺少文件数据".to_string());
    }
    let output_path = data_dir_relative_file_path(&state.config.data_dir, &output, "画布音频文件")?;
    let asset = state
        .media_asset
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
                source_external_path: Some(source.file_name.clone()),
                imported_from_user: true,
                request_snapshot_json: Some(json!({
                    "source": "video_canvas",
                    "canvas_import": true,
                    "media_kind": "audio",
                    "file_name": source.file_name,
                    "mime_type": input.mime_type,
                    "imported_at": Utc::now().to_rfc3339(),
                })),
            },
        })
        .await
        .map_err(|err| {
            let _ = std::fs::remove_file(&output);
            err.to_string()
        })?;
    Ok(asset)
}

struct CanvasAudioImportSource {
    file_name: String,
    extension: &'static str,
    path: Option<PathBuf>,
    bytes: Option<Vec<u8>>,
}

impl CanvasAudioImportSource {
    fn from_input(input: &ImportCanvasMediaInput) -> Result<Self, String> {
        if let Some(data_base64) = input.data_base64.as_deref() {
            return Self::from_upload(
                input
                    .file_name
                    .as_deref()
                    .ok_or_else(|| "导入画布音频缺少 file_name".to_string())?,
                input
                    .mime_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                data_base64,
            );
        }
        let file_path = input
            .file_path
            .as_deref()
            .ok_or_else(|| "导入画布音频缺少 file_path 或 data_base64".to_string())?;
        Self::from_path(file_path)
    }

    fn from_upload(file_name: &str, mime_type: &str, data_base64: &str) -> Result<Self, String> {
        let file_name = required_canvas_audio_file_name(file_name)?;
        let extension = canvas_audio_extension(Path::new(&file_name))?;
        validate_canvas_audio_mime(mime_type, extension)?;
        let raw = data_base64
            .split_once(";base64,")
            .map(|(_, body)| body)
            .unwrap_or(data_base64)
            .trim();
        if raw.is_empty() {
            return Err("导入画布音频数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("导入画布音频 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("导入画布音频文件不能为空".to_string());
        }
        if bytes.len() as u64 > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        Ok(Self {
            file_name,
            extension,
            path: None,
            bytes: Some(bytes),
        })
    }

    fn from_path(file_path: &str) -> Result<Self, String> {
        let trimmed = file_path.trim();
        if trimmed.is_empty() {
            return Err("请选择要导入的音频文件".to_string());
        }
        let path = PathBuf::from(trimmed);
        let metadata =
            std::fs::metadata(&path).map_err(|err| format!("无法读取导入音频文件：{err}"))?;
        if !metadata.is_file() {
            return Err("导入音频路径不是文件".to_string());
        }
        if metadata.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        let canonical = path
            .canonicalize()
            .map_err(|err| format!("无法解析导入音频路径：{err}"))?;
        let file_name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "audio".to_string());
        let extension = canvas_audio_extension(&canonical)?;
        Ok(Self {
            file_name,
            extension,
            path: Some(canonical),
            bytes: None,
        })
    }
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

fn required_canvas_audio_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("上传音频文件名不能为空".to_string());
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传音频文件名无效".to_string())?;
    if name != trimmed {
        return Err("上传音频文件名不能包含路径".to_string());
    }
    Ok(name.to_string())
}

fn canvas_audio_extension(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("导入音频缺少文件扩展名：{}", path.display()))?;
    match extension.to_ascii_lowercase().as_str() {
        "mp3" => Ok("mp3"),
        "wav" => Ok("wav"),
        "ogg" => Ok("ogg"),
        "m4a" => Ok("m4a"),
        "aac" => Ok("aac"),
        "flac" => Ok("flac"),
        _ => Err(format!("导入音频格式不支持：{extension}")),
    }
}

fn validate_canvas_audio_mime(mime_type: &str, extension: &str) -> Result<(), String> {
    let mime = mime_type.trim().to_ascii_lowercase();
    if mime.is_empty() || mime == "application/octet-stream" || mime.starts_with("audio/") {
        return Ok(());
    }
    let valid = match extension {
        "m4a" => mime == "video/mp4" || mime == "audio/mp4",
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("上传音频 MIME 与文件扩展名不匹配".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_audio_extension_accepts_supported_formats() {
        assert_eq!(
            canvas_audio_extension(Path::new("voice.mp3")).unwrap(),
            "mp3"
        );
        assert_eq!(
            canvas_audio_extension(Path::new("music.WAV")).unwrap(),
            "wav"
        );
        assert_eq!(
            canvas_audio_extension(Path::new("fx.flac")).unwrap(),
            "flac"
        );
    }

    #[test]
    fn canvas_audio_extension_rejects_unsupported_formats() {
        let err = canvas_audio_extension(Path::new("clip.mp4"))
            .expect_err("video extension must not be accepted as audio");

        assert!(err.contains("格式不支持"));
    }

    #[test]
    fn required_canvas_audio_file_name_rejects_paths() {
        let err = required_canvas_audio_file_name("../voice.mp3")
            .expect_err("upload file name must not contain a path");

        assert!(err.contains("不能包含路径"));
    }

    #[test]
    fn validate_canvas_audio_mime_accepts_audio_and_octet_stream() {
        assert!(validate_canvas_audio_mime("audio/mpeg", "mp3").is_ok());
        assert!(validate_canvas_audio_mime("application/octet-stream", "wav").is_ok());
        assert!(validate_canvas_audio_mime("video/mp4", "m4a").is_ok());
    }

    #[test]
    fn validate_canvas_audio_mime_rejects_non_audio_mime() {
        let err = validate_canvas_audio_mime("video/mp4", "mp3")
            .expect_err("video MIME must not be accepted for mp3");

        assert!(err.contains("MIME"));
    }
}
