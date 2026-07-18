use std::{fs, path::PathBuf};

use a4n_orchestrator::canvas::{
    VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter,
};

use crate::{file_utils::path_to_db_string, state::AppState};

use super::super::{ImportMediaPayload, media_file_key, required_package_extension};

pub(super) struct RuntimeVideoStudioImportMediaFileWriter<'a> {
    state: &'a AppState,
    media_payload: &'a ImportMediaPayload,
    written_files: &'a mut Vec<PathBuf>,
}

impl<'a> RuntimeVideoStudioImportMediaFileWriter<'a> {
    pub(super) fn new(
        state: &'a AppState,
        media_payload: &'a ImportMediaPayload,
        written_files: &'a mut Vec<PathBuf>,
    ) -> Self {
        Self {
            state,
            media_payload,
            written_files,
        }
    }
}

impl VideoStudioProjectImportMediaFileWriter for RuntimeVideoStudioImportMediaFileWriter<'_> {
    fn write_imported_media_file(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        owner_id: i64,
        field: &str,
        folder: &str,
        stem: &str,
    ) -> Result<Option<String>, VideoStudioProjectImportMediaError> {
        write_imported_media_file(
            self.state,
            project_id,
            self.media_payload,
            owner_kind,
            owner_id,
            field,
            folder,
            stem,
            self.written_files,
        )
        .map(|path| path.map(|path| path_to_db_string(&path)))
        .map_err(VideoStudioProjectImportMediaError::Write)
    }
}

fn write_imported_media_file(
    state: &AppState,
    project_id: i64,
    media_payload: &ImportMediaPayload,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    folder: &str,
    stem: &str,
    written_files: &mut Vec<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let key = media_file_key(owner_kind, owner_id, field);
    let Some(entry) = media_payload.entries.get(&key) else {
        return Ok(None);
    };
    let bytes = media_payload.bytes.get(&entry.package_path).ok_or_else(|| {
        format!(
            "视频画布项目包缺少媒体内容: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
        )
    })?;
    let extension = required_package_extension(&entry.package_path)?;
    let relative = PathBuf::from(format!(
        "projects/{project_id}/{folder}/{stem}_{owner_id}_{field}.{extension}"
    ));
    let path = state.config.data_dir.join(&relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建媒体导入目录: {err}"))?;
    }
    fs::write(&path, bytes).map_err(|err| format!("写入导入媒体文件失败: {err}"))?;
    written_files.push(path);
    Ok(Some(relative))
}
