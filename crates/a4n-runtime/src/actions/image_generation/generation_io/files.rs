use crate::file_utils::persist_created_media_file;
use a4n_core::files::paths::ProjectPaths;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub(super) fn persist_generated_image_file(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    kind: &str,
    extra: Option<&str>,
    bytes: &[u8],
    format: &str,
    file_context: &str,
) -> Result<(PathBuf, String, i64), String> {
    let kind = required_filename_segment(kind, "kind")?;
    let format = required_filename_segment(format, "format")?;
    let suffix = match extra {
        Some(value) => format!("_{}", required_filename_segment(value, "extra")?),
        None => String::new(),
    };
    let (path, mut output_file) =
        create_image_output(data_dir, project_id, chapter_id, kind, &suffix, format)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(bytes)
                .map_err(|error| format!("无法写入图片：{error}"))
        },
        file_context,
        file_context,
        file_context,
    )?;
    Ok((path, relative_path, file_size))
}

fn create_image_output(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    kind: &str,
    suffix: &str,
    format: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let dir = image_output_dir(data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建图片目录 {dir:?}：{e}"))?;
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let collision_suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let filename = format!("{kind}{suffix}_{timestamp}{collision_suffix}.{format}");
        let path = dir.join(filename);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("无法创建图片文件：{error}")),
        }
    }
    Err("无法创建图片文件：文件名冲突过多".to_string())
}

fn image_output_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<PathBuf, String> {
    let project_paths = ProjectPaths::new(data_dir, project_id);
    match chapter_id {
        Some(chapter_id) if chapter_id > 0 => Ok(project_paths.chapter_images_dir(chapter_id)),
        Some(chapter_id) => Err(format!(
            "章节图片目录需要有效 chapter_id，收到 {chapter_id}"
        )),
        None => Ok(project_paths.images_dir()),
    }
}

pub(super) fn required_filename_segment<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("图片文件名片段 {field} 不能为空"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(format!(
            "图片文件名片段 {field} 只能包含 ASCII 字母、数字、下划线或短横线"
        ));
    }
    Ok(value)
}
