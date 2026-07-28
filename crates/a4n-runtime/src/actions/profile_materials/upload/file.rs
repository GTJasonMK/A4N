use crate::file_utils::persist_created_media_file;
use chrono::Utc;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(in crate::actions::profile_materials) fn persist_uploaded_profile_material_image(
    data_dir: &Path,
    project_id: i64,
    kind: &str,
    profile_id: i64,
    extension: &str,
    bytes: &[u8],
) -> Result<(PathBuf, String, i64), String> {
    let (path, mut output_file) =
        create_uploaded_profile_material_output(data_dir, project_id, kind, profile_id, extension)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(bytes)
                .map_err(|err| format!("写入上传素材图失败：{err}"))
        },
        "上传素材图文件",
        "上传素材图",
        "上传素材图文件",
    )?;
    Ok((path, relative_path, file_size))
}

fn create_uploaded_profile_material_output(
    data_dir: &Path,
    project_id: i64,
    kind: &str,
    profile_id: i64,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let dir = data_dir
        .join("projects")
        .join(project_id.to_string())
        .join("images")
        .join("profile_materials");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建素材图目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let file_name = format!("{kind}_{profile_id}_{stamp}{suffix}.{extension}");
        let path = dir.join(file_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("创建上传素材图失败：{err}")),
        }
    }
    Err("创建上传素材图失败：文件名冲突过多".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_profile_material_under_project_directory() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!(
            "a4n_profile_material_{}_{}",
            std::process::id(),
            stamp
        ));
        let (path, relative_path, file_size) = persist_uploaded_profile_material_image(
            &data_dir,
            7,
            "scene",
            42,
            "png",
            b"profile-material",
        )
        .expect("persist profile material");

        assert!(path.is_file());
        assert!(relative_path.starts_with("projects/7/images/profile_materials/scene_42_"));
        assert_eq!(file_size, 16);
        let _ = std::fs::remove_dir_all(data_dir);
    }
}
