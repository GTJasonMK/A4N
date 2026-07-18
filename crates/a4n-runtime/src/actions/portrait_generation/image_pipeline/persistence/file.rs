use std::io::Write;

use a4n_core::files::paths::ProjectPaths;

use crate::file_utils::persist_created_media_file;

pub(in crate::actions::portrait_generation) fn persist_portrait_file(
    data_dir: &std::path::Path,
    project_id: i64,
    profile_id: i64,
    bytes: &[u8],
    format: &str,
    path_context: &str,
) -> Result<(std::path::PathBuf, String, i64), String> {
    let (path, mut output_file) = create_portrait_output(data_dir, project_id, profile_id, format)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(bytes)
                .map_err(|error| format!("无法写入立绘：{error}"))
        },
        path_context,
        path_context,
        path_context,
    )?;
    Ok((path, relative_path, file_size))
}

fn create_portrait_output(
    data_dir: &std::path::Path,
    project_id: i64,
    profile_id: i64,
    format: &str,
) -> Result<(std::path::PathBuf, std::fs::File), String> {
    let dir = ProjectPaths::new(data_dir, project_id).portraits_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建立绘目录：{e}"))?;
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!(
            "portrait_{profile_id}_{timestamp}{suffix}.{format}"
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("无法创建立绘文件：{err}")),
        }
    }
    Err("无法创建立绘文件：文件名冲突过多".to_string())
}
