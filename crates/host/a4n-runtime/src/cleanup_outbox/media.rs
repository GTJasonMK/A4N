use crate::state::RuntimeProfile;
use a4n_storage::remove_data_file_in_data_dir;
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

#[cfg(test)]
mod tests {
    use super::remove_media_paths;

    #[cfg(unix)]
    #[test]
    fn remove_media_paths_preserves_explicit_unlink_failure() {
        use std::os::unix::fs::PermissionsExt;

        let data_dir = tempfile::tempdir().expect("create cleanup data directory");
        let media_dir = data_dir.path().join("projects/1/media");
        let media_path = media_dir.join("unlink-failure.bin");
        std::fs::create_dir_all(&media_dir).expect("create media directory");
        std::fs::write(&media_path, b"media").expect("write media file");

        let mut permissions = std::fs::metadata(&media_dir)
            .expect("read media directory permissions")
            .permissions();
        let original_mode = permissions.mode();
        permissions.set_mode(0o555);
        std::fs::set_permissions(&media_dir, permissions).expect("block media directory unlink");

        let result = remove_media_paths(
            data_dir.path(),
            vec!["projects/1/media/unlink-failure.bin".to_string()],
        );

        let mut restored_permissions = std::fs::metadata(&media_dir)
            .expect("read media directory permissions after cleanup")
            .permissions();
        restored_permissions.set_mode(original_mode);
        std::fs::set_permissions(&media_dir, restored_permissions)
            .expect("restore media directory permissions");

        let error = result.expect_err("unlink failure must remain explicit");
        assert!(error.contains("项目删除 cleanup 媒体文件删除失败"));
        assert!(media_path.is_file(), "failed unlink must preserve the file");
    }
}
