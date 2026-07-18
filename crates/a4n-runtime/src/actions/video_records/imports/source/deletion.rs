use super::super::prelude::{AppState, ErrorKind, Future, Path, PathBuf, deletable_data_file_path};

pub(in crate::actions::video_records) async fn delete_video_record_then_files<F, Fut>(
    file_paths: Vec<(&'static str, PathBuf)>,
    delete_record: F,
) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    delete_record().await?;
    remove_video_delete_files(&file_paths)
}

pub(in crate::actions::video_records) async fn delete_video_record_error(
    state: &AppState,
    record_id: i64,
) -> Option<String> {
    state
        .orchestrator
        .delete_generated_video_record(record_id)
        .await
        .err()
        .map(crate::actions::app_error_message)
}

pub(in crate::actions::video_records) fn video_delete_file_paths(
    data_dir: &Path,
    file_path: Option<&str>,
    thumbnail_path: Option<&str>,
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    let mut out = Vec::new();
    for (label, path) in [("视频文件", file_path), ("视频缩略图", thumbnail_path)] {
        let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
            continue;
        };
        let Some(canonical) = deletable_data_file_path(data_dir, path, label)? else {
            continue;
        };
        if !out
            .iter()
            .any(|(_, existing_path)| existing_path == &canonical)
        {
            out.push((label, canonical));
        }
    }
    Ok(out)
}

fn remove_video_delete_files(file_paths: &[(&'static str, PathBuf)]) -> Result<(), String> {
    for (label, path) in file_paths {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(format!("{label}删除失败：{}：{err}", path.display())),
        }
    }
    Ok(())
}
