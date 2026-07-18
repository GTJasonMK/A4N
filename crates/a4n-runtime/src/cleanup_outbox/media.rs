use crate::file_utils::remove_data_file_in_data_dir;
use crate::state::RuntimeProfile;
use std::path::Path;

pub(crate) fn project_media_path_prefix(project_id: i64) -> String {
    format!("projects/{project_id}/")
}

pub(super) fn cleanup_requires_rag(profile: RuntimeProfile) -> bool {
    profile.initializes_rag() || profile.uses_sidecars()
}

pub(super) fn remove_media_paths(data_dir: &Path, paths: Vec<String>) -> Result<(), String> {
    let mut errors = Vec::new();
    for path in paths {
        if let Err(error) =
            remove_data_file_in_data_dir(data_dir, &path, "项目删除 cleanup 媒体文件")
        {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("；"))
    }
}
