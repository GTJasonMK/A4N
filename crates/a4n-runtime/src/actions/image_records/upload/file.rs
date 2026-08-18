use a4n_core::files::{create_unique_timestamped_file, paths::ProjectPaths};
use std::path::{Path, PathBuf};

pub(super) fn create_imported_image_output(
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
    create_unique_timestamped_file(&dir, "imported_image", extension)
        .map_err(|error| format!("创建导入图片失败：{error}"))
}
