use std::path::Path;

use crate::file_utils::deletable_data_file_path;

pub(in crate::workflow::chapter_cleanup) fn validate_chapter_media_file_paths(
    data_dir: &Path,
    paths: &[String],
) -> Result<(), String> {
    for path in paths {
        deletable_data_file_path(data_dir, path, "章节媒体文件")?;
    }
    Ok(())
}
