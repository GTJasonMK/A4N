use a4n_storage::paths::ProjectPaths;
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
