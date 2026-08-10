use a4n_core::{
    error::AppError,
    files::paths::ProjectPaths,
    models::{
        ai::{generated_image, generated_video},
        media::media_asset,
    },
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{
    OrchestratorContext,
    canvas::{CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput},
    media::{ImportedImageUploadInput, MEDIA_ORIGIN_IMPORTED},
    project::PROJECT_KIND_VIDEO,
};

#[derive(Debug, Clone)]
pub struct CanvasImageImportHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoImportHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
    pub prompt: Option<String>,
}

#[async_trait]
pub trait CanvasVideoImportHostPort: Send + Sync {
    async fn import_canvas_video(
        &self,
        input: CanvasVideoImportHostInput,
    ) -> Result<generated_video::Model, AppError>;

    async fn delete_canvas_video(&self, video_id: i64) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct CanvasImageImportApplicationResult {
    pub image: generated_image::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoImportApplicationResult {
    pub video: generated_video::Model,
    pub asset: media_asset::Model,
}

impl OrchestratorContext {
    pub async fn import_canvas_image_validated(
        &self,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project_id = input.project_id;
        let user_id = input.user_id;
        self.require_canvas_video_project(project_id).await?;
        let prepared = self
            .prepare_imported_image_upload_for_user_validated(
                user_id,
                project_id,
                ImportedImageUploadInput {
                    file_name: input.file_name,
                    mime_type: input.mime_type,
                    data_base64: input.data_base64,
                    image_type: "canvas_image".to_string(),
                    prompt: input.prompt,
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
        let (relative_path, file_size) = persist_imported_canvas_image(
            self.data_dir(),
            prepared.project_id,
            prepared.chapter_id,
            prepared.decoded.extension,
            &prepared.decoded.bytes,
        )?;
        let image = match self
            .create_imported_image_record_for_user_validated(
                user_id,
                prepared,
                relative_path.clone(),
                file_size,
            )
            .await
        {
            Ok(image) => image,
            Err(error) => {
                let _ = remove_data_file(self.data_dir(), &relative_path);
                return Err(error);
            }
        };
        if image.project_id != project_id {
            let primary = AppError::InvalidState {
                current: format!("canvas_image_project_id={}", image.project_id),
                operation: format!("画布导入图片必须属于请求项目 {project_id}"),
            };
            if let Err(cleanup) = self
                .delete_generated_image_for_user_validated(user_id, image.id)
                .await
            {
                return Err(append_import_cleanup_error(
                    primary,
                    cleanup,
                    "画布导入图片",
                ));
            }
            return Err(primary);
        }
        let asset = match self
            .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
                project_id,
                image: image.clone(),
                generation_spec_id: None,
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            })
            .await
        {
            Ok(asset) => asset,
            Err(primary) => {
                if let Err(cleanup) = self
                    .delete_generated_image_for_user_validated(user_id, image.id)
                    .await
                {
                    return Err(append_import_cleanup_error(
                        primary,
                        cleanup,
                        "画布导入图片",
                    ));
                }
                return Err(primary);
            }
        };
        Ok(CanvasImageImportApplicationResult { image, asset })
    }

    pub async fn import_canvas_video_validated(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project_id = input.project_id;
        self.require_canvas_video_project(project_id).await?;
        let video = host_port.import_canvas_video(input).await?;
        if video.project_id != project_id {
            let primary = AppError::InvalidState {
                current: format!("canvas_video_project_id={}", video.project_id),
                operation: format!("画布导入视频必须属于请求项目 {project_id}"),
            };
            if let Err(cleanup) = host_port.delete_canvas_video(video.id).await {
                return Err(append_import_cleanup_error(
                    primary,
                    cleanup,
                    "画布导入视频",
                ));
            }
            return Err(primary);
        }
        let asset = match self
            .ensure_canvas_video_media_asset_record(CanvasVideoMediaAssetEnsureInput {
                project_id,
                video: video.clone(),
                generation_spec_id: None,
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            })
            .await
        {
            Ok(asset) => asset,
            Err(primary) => {
                if let Err(cleanup) = host_port.delete_canvas_video(video.id).await {
                    return Err(append_import_cleanup_error(
                        primary,
                        cleanup,
                        "画布导入视频",
                    ));
                }
                return Err(primary);
            }
        };
        Ok(CanvasVideoImportApplicationResult { video, asset })
    }

    async fn require_canvas_video_project(&self, project_id: i64) -> Result<(), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "项目 ID 必须是正整数".to_string(),
            ));
        }
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_VIDEO {
            return Err(AppError::InvalidArgument(
                "画布媒体导入只支持视频项目".to_string(),
            ));
        }
        Ok(())
    }
}

fn append_import_cleanup_error(primary: AppError, cleanup: AppError, label: &str) -> AppError {
    AppError::InvalidState {
        current: primary.to_string(),
        operation: format!("{label}媒体资产登记失败后的补偿删除也失败：{cleanup}"),
    }
}

fn persist_imported_canvas_image(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    extension: &str,
    bytes: &[u8],
) -> Result<(String, i64), AppError> {
    let (path, mut output_file) =
        create_imported_image_output(data_dir, project_id, chapter_id, extension).map_err(
            |operation| AppError::InvalidState {
                current: "canvas_image_import".to_string(),
                operation,
            },
        )?;
    if let Err(error) = output_file.write_all(bytes) {
        let _ = std::fs::remove_file(&path);
        return Err(AppError::InvalidState {
            current: "canvas_image_import".to_string(),
            operation: format!("写入导入图片失败：{error}"),
        });
    }
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            let _ = std::fs::remove_file(&path);
            return Err(AppError::InvalidState {
                current: "canvas_image_import".to_string(),
                operation: "导入图片不是文件".to_string(),
            });
        }
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            return Err(AppError::InvalidState {
                current: "canvas_image_import".to_string(),
                operation: format!("无法读取导入图片元数据：{error}"),
            });
        }
    };
    let file_size = i64::try_from(metadata.len()).map_err(|_| AppError::InvalidState {
        current: "canvas_image_import".to_string(),
        operation: "导入图片过大，无法记录文件大小".to_string(),
    })?;
    let relative_path = data_dir_relative_path(data_dir, &path).map_err(|operation| {
        let _ = std::fs::remove_file(&path);
        AppError::InvalidState {
            current: "canvas_image_import".to_string(),
            operation,
        }
    })?;
    Ok((relative_path, file_size))
}

fn create_imported_image_output(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let dir = match chapter_id {
        Some(chapter_id) if chapter_id > 0 => {
            ProjectPaths::new(data_dir, project_id).chapter_images_dir(chapter_id)
        }
        Some(chapter_id) => {
            return Err(format!(
                "导入章节图片需要有效 chapter_id，收到 {chapter_id}"
            ));
        }
        None => ProjectPaths::new(data_dir, project_id).images_dir(),
    };
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建导入图片目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("imported_image_{stamp}{suffix}.{extension}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("创建导入图片失败：{err}")),
        }
    }
    Err("创建导入图片失败：文件名冲突过多".to_string())
}

fn data_dir_relative_path(data_dir: &Path, path: &Path) -> Result<String, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("解析应用数据目录失败：{error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("解析文件路径失败：{error}"))?;
    canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| "导入图片不在应用数据目录内".to_string())?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "文件相对路径包含非 UTF-8 字符".to_string())
}

fn remove_data_file(data_dir: &Path, relative_path: &str) -> Result<(), String> {
    let raw = Path::new(relative_path);
    if raw.is_absolute() {
        return Err("导入图片路径必须是相对路径".to_string());
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("解析应用数据目录失败：{error}"))?;
    let path = match data_dir.join(raw).canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("解析导入图片路径失败：{error}")),
    };
    if !path.starts_with(&canonical_data_dir) {
        return Err("导入图片路径超出应用数据目录".to_string());
    }
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|error| format!("删除导入图片失败：{error}"))?;
    }
    Ok(())
}
