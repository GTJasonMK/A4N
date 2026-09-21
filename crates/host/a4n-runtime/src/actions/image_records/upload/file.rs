use a4n_storage::paths::{ProjectPaths, path_to_db_string};
use std::path::{Path, PathBuf};

pub(super) fn create_imported_image_output(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    extension: &str,
    stem: &str,
) -> Result<(PathBuf, PathBuf), String> {
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
    if stem.is_empty()
        || !stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || extension.is_empty()
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("导入图片文件名片段无效".to_string());
    }
    let final_path = dir.join(format!("{stem}.{extension}"));
    let staging_path = dir.join(format!("{stem}.{extension}.part"));
    if final_path.exists() || staging_path.exists() {
        return Err("导入图片发布路径已存在".to_string());
    }
    Ok((staging_path, final_path))
}

pub(super) fn data_path_to_db_string(data_dir: &Path, path: &Path) -> Result<String, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "导入图片路径缺少文件名".to_string())?;
    let canonical_path = path
        .parent()
        .ok_or_else(|| "导入图片路径缺少父目录".to_string())?
        .canonicalize()
        .map_err(|error| format!("无法解析导入图片目录：{error}"))?
        .join(file_name);
    let relative = canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| "导入图片路径不在数据目录内".to_string())?;
    path_to_db_string(relative)
}
