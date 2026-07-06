use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

pub fn canonicalize_path_in_data_dir(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("无法读取{label}：{e}"))?;
    if !canonical_path.starts_with(&canonical_data_dir) {
        return Err(format!("{label}不在应用数据目录内"));
    }
    Ok(canonical_path)
}

pub fn canonicalize_data_file_path(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    let raw_path = Path::new(trimmed);
    if raw_path.is_absolute() {
        return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
    }
    let candidate = data_dir.join(raw_path);
    let canonical_path = canonicalize_path_in_data_dir(data_dir, &candidate, label)?;
    let metadata =
        std::fs::metadata(&canonical_path).map_err(|e| format!("无法读取{label}元数据：{e}"))?;
    if !metadata.is_file() {
        return Err(format!("{label}不是文件"));
    }
    Ok(canonical_path)
}

pub fn data_dir_relative_file_path(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<String, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let canonical_path = canonicalize_path_in_data_dir(data_dir, path, label)?;
    canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| format!("{label}不在应用数据目录内"))
        .map(path_to_db_string)
}

pub fn path_to_db_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn remove_data_file_in_data_dir(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_file_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_file(&canonical_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{label}删除失败：{}：{err}",
            canonical_path.display()
        )),
    }
}

pub fn remove_data_dir_in_data_dir(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_dir_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_dir_all(&canonical_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{label}删除失败：{}：{err}",
            canonical_path.display()
        )),
    }
}

pub fn deletable_data_file_path(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    let raw_path = Path::new(trimmed);
    if raw_path.is_absolute() {
        return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
    }
    let relative_path = normalize_relative_data_path(raw_path, label)?;
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let candidate = canonical_data_dir.join(relative_path);
    match std::fs::metadata(&candidate) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(format!("{label}不是文件"));
            }
            canonicalize_path_in_data_dir(&canonical_data_dir, &candidate, label).map(Some)
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            validate_missing_delete_target_parent(&canonical_data_dir, &candidate, label)?;
            Ok(None)
        }
        Err(err) => Err(format!("无法读取{label}元数据：{err}")),
    }
}

pub fn deletable_data_dir_path(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_data_dir.join(normalize_relative_data_path(path, label)?)
    };
    match std::fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!("{label}不能是符号链接"));
            }
            if !metadata.is_dir() {
                return Err(format!("{label}不是目录"));
            }
            let canonical_path =
                canonicalize_path_in_data_dir(&canonical_data_dir, &candidate, label)?;
            if canonical_path == canonical_data_dir {
                return Err(format!("{label}不能是应用数据目录"));
            }
            Ok(Some(canonical_path))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            validate_missing_delete_target_parent(&canonical_data_dir, &candidate, label)?;
            Ok(None)
        }
        Err(err) => Err(format!("无法读取{label}元数据：{err}")),
    }
}

fn normalize_relative_data_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => out.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err(format!("{label}不在应用数据目录内"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    Ok(out)
}

fn validate_missing_delete_target_parent(
    canonical_data_dir: &Path,
    candidate: &Path,
    label: &str,
) -> Result<(), String> {
    for ancestor in candidate.ancestors().skip(1) {
        match std::fs::metadata(ancestor) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(format!("{label}父路径不是目录"));
                }
                let canonical_ancestor = ancestor
                    .canonicalize()
                    .map_err(|err| format!("无法解析{label}父路径：{err}"))?;
                if !canonical_ancestor.starts_with(canonical_data_dir) {
                    return Err(format!("{label}不在应用数据目录内"));
                }
                return Ok(());
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(format!("无法读取{label}父路径：{err}")),
        }
    }
    Err(format!("{label}不在应用数据目录内"))
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_data_file_path, canonicalize_path_in_data_dir, deletable_data_dir_path,
        remove_data_dir_in_data_dir, remove_data_file_in_data_dir,
    };
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .expect("system clock must be after UNIX_EPOCH");
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
    }

    #[test]
    fn canonicalize_path_in_data_dir_accepts_nested_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_ok");
        let file_path = data_dir.join("projects/1/videos/chapter_1/video_1.mp4");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");

        let resolved = canonicalize_path_in_data_dir(&data_dir, &file_path, "视频文件")
            .expect("resolve nested file");
        assert_eq!(
            resolved,
            file_path.canonicalize().expect("canonicalize file")
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_data_file_path_accepts_data_dir_relative_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_relative_ok");
        let file_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");

        let resolved =
            canonicalize_data_file_path(&data_dir, "projects/1/images/ref.png", "图片文件")
                .expect("resolve relative data file");
        assert_eq!(
            resolved,
            file_path.canonicalize().expect("canonicalize file")
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_data_file_path_rejects_blank_path() {
        let data_dir = unique_temp_dir("a4n_file_utils_blank");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let err = canonicalize_data_file_path(&data_dir, " \n\t ", "图片文件")
            .expect_err("blank path should fail before filesystem lookup");
        assert_eq!(err, "图片文件路径不能为空");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_data_file_path_rejects_directory() {
        let data_dir = unique_temp_dir("a4n_file_utils_directory");
        let dir_path = data_dir.join("projects/1/images/not_a_file");
        std::fs::create_dir_all(&dir_path).expect("create nested dir");

        let err =
            canonicalize_data_file_path(&data_dir, "projects/1/images/not_a_file", "图片文件")
                .expect_err("directory path must not be accepted as file");

        assert_eq!(err, "图片文件不是文件");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_data_file_path_rejects_absolute_path() {
        let data_dir = unique_temp_dir("a4n_file_utils_absolute_data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let file_path = data_dir.join("projects/1/images/ref.png");

        let err = canonicalize_data_file_path(&data_dir, &file_path.to_string_lossy(), "图片文件")
            .expect_err("absolute data file path should fail");
        assert!(err.contains("相对路径"));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_path_in_data_dir_rejects_outside_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_bad");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let outside = std::env::temp_dir().join(format!(
            "outside_{}_{}.mp4",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .expect("system clock must be after UNIX_EPOCH")
        ));
        std::fs::write(&outside, b"demo").expect("write outside file");

        let err = canonicalize_path_in_data_dir(&data_dir, &outside, "视频文件")
            .expect_err("reject outside file");
        assert!(err.contains("不在应用数据目录内"));

        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn canonicalize_data_file_path_rejects_relative_traversal() {
        let root = unique_temp_dir("a4n_file_utils_traversal");
        let data_dir = root.join("data");
        let outside = root.join("secret.png");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        std::fs::write(&outside, b"demo").expect("write outside file");

        let err = canonicalize_data_file_path(&data_dir, "../secret.png", "图片文件")
            .expect_err("reject traversal outside data dir");
        assert!(err.contains("不在应用数据目录内"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remove_data_file_in_data_dir_removes_existing_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_ok");
        let file_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");

        remove_data_file_in_data_dir(&data_dir, "projects/1/images/ref.png", "图片文件")
            .expect("remove existing data file");

        assert!(!file_path.exists());
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_file_in_data_dir_skips_missing_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_missing");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        remove_data_file_in_data_dir(&data_dir, "projects/1/images/missing.png", "图片文件")
            .expect("missing delete target should be treated as already removed");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_file_in_data_dir_rejects_missing_traversal_path() {
        let root = unique_temp_dir("a4n_file_utils_remove_traversal");
        let data_dir = root.join("data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let err = remove_data_file_in_data_dir(&data_dir, "../missing.png", "图片文件")
            .expect_err("missing traversal path must still be rejected");
        assert!(err.contains("不在应用数据目录内"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remove_data_dir_in_data_dir_removes_existing_directory() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_ok");
        let project_dir = data_dir.join("projects/1");
        let nested_file = project_dir.join("images/ref.png");
        std::fs::create_dir_all(nested_file.parent().expect("nested parent"))
            .expect("create nested dir");
        std::fs::write(&nested_file, b"demo").expect("write nested file");

        remove_data_dir_in_data_dir(&data_dir, &project_dir, "项目数据目录")
            .expect("remove existing data directory");

        assert!(!project_dir.exists());
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_dir_in_data_dir_skips_missing_directory() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_missing");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        remove_data_dir_in_data_dir(
            &data_dir,
            PathBuf::from("projects/1").as_path(),
            "项目数据目录",
        )
        .expect("missing directory should be treated as already removed");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_dir_in_data_dir_rejects_file() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_file");
        let file_path = data_dir.join("projects/1/project.toml");
        std::fs::create_dir_all(file_path.parent().expect("nested parent"))
            .expect("create nested dir");
        std::fs::write(&file_path, b"demo").expect("write file");

        let err = remove_data_dir_in_data_dir(&data_dir, &file_path, "项目数据目录")
            .expect_err("file path must not be removed as directory");
        assert_eq!(err, "项目数据目录不是目录");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_dir_in_data_dir_rejects_data_dir_itself() {
        let data_dir = unique_temp_dir("a4n_file_utils_remove_data_dir");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let err = remove_data_dir_in_data_dir(&data_dir, &data_dir, "项目数据目录")
            .expect_err("data dir itself must not be removed");
        assert_eq!(err, "项目数据目录不能是应用数据目录");
        assert!(data_dir.exists());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn remove_data_dir_in_data_dir_rejects_outside_directory() {
        let root = unique_temp_dir("a4n_file_utils_remove_dir_outside");
        let data_dir = root.join("data");
        let outside_dir = root.join("outside");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        std::fs::create_dir_all(&outside_dir).expect("create outside dir");

        let err = remove_data_dir_in_data_dir(&data_dir, &outside_dir, "项目数据目录")
            .expect_err("outside directory must not be removed");
        assert!(err.contains("不在应用数据目录内"));
        assert!(outside_dir.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn remove_data_dir_in_data_dir_rejects_symlink_directory() {
        let root = unique_temp_dir("a4n_file_utils_remove_dir_symlink");
        let data_dir = root.join("data");
        let target_dir = data_dir.join("targets/project");
        let project_link = data_dir.join("projects/1");
        std::fs::create_dir_all(&target_dir).expect("create target dir");
        std::fs::create_dir_all(project_link.parent().expect("project parent"))
            .expect("create project parent");
        std::os::unix::fs::symlink(&target_dir, &project_link).expect("create directory symlink");

        let err = remove_data_dir_in_data_dir(&data_dir, &project_link, "项目数据目录")
            .expect_err("directory symlink must not be removed as project dir");
        assert_eq!(err, "项目数据目录不能是符号链接");
        assert!(target_dir.exists());
        assert!(project_link.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remove_data_dir_in_data_dir_rejects_missing_traversal_path() {
        let root = unique_temp_dir("a4n_file_utils_remove_dir_traversal");
        let data_dir = root.join("data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let err = remove_data_dir_in_data_dir(
            &data_dir,
            PathBuf::from("../missing_project").as_path(),
            "项目数据目录",
        )
        .expect_err("missing traversal path must still be rejected");
        assert!(err.contains("不在应用数据目录内"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deletable_data_dir_path_accepts_relative_project_directory() {
        let data_dir = unique_temp_dir("a4n_file_utils_dir_relative_ok");
        let project_dir = data_dir.join("projects/1");
        std::fs::create_dir_all(&project_dir).expect("create project dir");

        let resolved = deletable_data_dir_path(
            &data_dir,
            PathBuf::from("projects/1").as_path(),
            "项目数据目录",
        )
        .expect("relative data dir should resolve")
        .expect("existing data dir should be deletable");
        assert_eq!(
            resolved,
            project_dir
                .canonicalize()
                .expect("canonicalize project dir")
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
