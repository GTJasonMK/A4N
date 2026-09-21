use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::super::contracts::{ImportedVideoSource, MAX_UPLOAD_VIDEO_BYTES};
use super::import_video_source;
use a4n_media::{decode_uploaded_bytes, required_upload_file_name};
use a4n_storage::create_unique_timestamped_file;
use a4n_storage::remove_created_file_on_error_sync;
use a4n_video::{import_video_extension, validate_upload_video_mime};

const UPLOADED_VIDEO_TEMP_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub(crate) fn recover_expired_uploaded_video_sources(temp_dir: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(UPLOADED_VIDEO_TEMP_TTL) else {
        tracing::error!("当前系统时间无法计算上传视频临时文件过期边界");
        return;
    };
    let paths = match expired_uploaded_video_sources(temp_dir, cutoff) {
        Ok(paths) => paths,
        Err(error) => {
            tracing::warn!(%error, "扫描过期上传视频临时文件失败");
            return;
        }
    };
    for path in paths {
        if let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), %error, "清理过期上传视频临时文件失败");
        }
    }
}

pub(in crate::actions::video_records) fn expired_uploaded_video_sources(
    temp_dir: &Path,
    cutoff: SystemTime,
) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(temp_dir.join("video_uploads")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("读取视频上传临时目录失败：{error}")),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("读取视频上传临时目录项失败：{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("读取视频上传临时文件类型失败：{error}"))?
            .is_file()
        {
            continue;
        }
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("video_upload_"))
        {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|error| format!("读取视频上传临时文件时间失败：{error}"))?;
        if modified <= cutoff {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub(in crate::actions::video_records) fn uploaded_video_source(
    temp_dir: &Path,
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
    label: &str,
) -> Result<ImportedVideoSource, String> {
    let file_name = required_upload_file_name(file_name, "上传视频文件名")?;
    let extension = import_video_extension(Path::new(&file_name))?;
    validate_upload_video_mime(mime_type, extension)?;
    let bytes = decode_uploaded_bytes(data_base64, label, MAX_UPLOAD_VIDEO_BYTES)?;

    persist_uploaded_video_source(temp_dir, file_name, extension, &bytes, label)
}

pub(in crate::actions::video_records) fn persist_uploaded_video_source(
    temp_dir: &Path,
    file_name: String,
    extension: &'static str,
    bytes: &[u8],
    label: &str,
) -> Result<ImportedVideoSource, String> {
    let (temp_path, mut output_file) = create_uploaded_video_temp_file(temp_dir, extension)?;
    let file_label = format!("{label}临时文件");
    remove_created_file_on_error_sync(&temp_path, &file_label, || {
        output_file
            .write_all(bytes)
            .map_err(|error| format!("写入{label}临时文件失败：{error}"))?;
        let mut source = import_video_source(
            temp_path
                .to_str()
                .ok_or_else(|| "视频上传临时路径不是有效 UTF-8".to_string())?,
            label,
        )?;
        source.file_name = file_name;
        source.temporary_path = Some(temp_path.clone());
        Ok(source)
    })
}

fn create_uploaded_video_temp_file(
    temp_dir: &Path,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let upload_dir = temp_dir.join("video_uploads");
    std::fs::create_dir_all(&upload_dir).map_err(|e| format!("创建视频上传临时目录失败：{e}"))?;
    create_unique_timestamped_file(&upload_dir, "video_upload", extension)
        .map_err(|error| format!("创建视频上传临时文件失败：{error}"))
}
