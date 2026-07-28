use std::io::Write;
use std::time::{Duration, SystemTime};

use super::super::contracts::{ImportedVideoSource, MAX_UPLOAD_VIDEO_BYTES};
use super::super::prelude::*;
use super::import_video_source;
use a4n_orchestrator::media::strip_data_url_prefix;
use a4n_orchestrator::video::{
    import_video_extension, required_upload_video_file_name, validate_upload_video_mime,
};

const UPLOADED_VIDEO_TEMP_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub(crate) fn recover_expired_uploaded_video_sources(temp_dir: &Path) {
    let cutoff = SystemTime::now()
        .checked_sub(UPLOADED_VIDEO_TEMP_TTL)
        .unwrap_or(SystemTime::UNIX_EPOCH);
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
    let file_name = required_upload_video_file_name(file_name)?;
    let extension = import_video_extension(Path::new(&file_name))?;
    validate_upload_video_mime(mime_type, extension)?;
    let raw = strip_data_url_prefix(data_base64).trim();
    if raw.is_empty() {
        return Err(format!("{label}数据不能为空"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("{label} base64 解码失败：{err}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}文件不能为空"));
    }
    if bytes.len() > MAX_UPLOAD_VIDEO_BYTES {
        return Err(format!(
            "{label}不能超过 {}MB",
            MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024
        ));
    }

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
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let temp_path = upload_dir.join(format!(
            "video_upload_{}_{}{suffix}.{}",
            std::process::id(),
            stamp,
            extension
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("创建视频上传临时文件失败：{error}")),
        }
    }
    Err("创建视频上传临时文件失败：文件名冲突过多".to_string())
}
