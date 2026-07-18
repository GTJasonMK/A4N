use super::super::{
    ImportMediaPayload, MEDIA_FILE_FIELD_THUMBNAIL, media_asset_path, media_file_key,
    required_package_extension,
};
use a4n_core::models::media::media_asset;
use a4n_orchestrator::project::{ProjectFamilyMediaFileImporter, ProjectFamilyMediaImportError};
use std::path::{Path, PathBuf};

#[cfg(test)]
pub(in crate::actions::project_transfer::family_package) use a4n_orchestrator::project::media_asset_status_for_import_insert;

pub(in crate::actions::project_transfer::family_package) fn cleanup_imported_media_files(
    primary: String,
    paths: &[PathBuf],
) -> String {
    crate::file_utils::error_with_rollback_file_cleanup(primary, paths, "已导入的项目族媒体文件")
}

pub(in crate::actions::project_transfer::family_package) struct RuntimeProjectFamilyMediaFileImporter<
    'a,
> {
    data_dir: &'a Path,
    media_payload: &'a ImportMediaPayload,
    written_files: &'a mut Vec<PathBuf>,
}

impl<'a> RuntimeProjectFamilyMediaFileImporter<'a> {
    pub(in crate::actions::project_transfer::family_package) fn new(
        data_dir: &'a Path,
        media_payload: &'a ImportMediaPayload,
        written_files: &'a mut Vec<PathBuf>,
    ) -> Self {
        Self {
            data_dir,
            media_payload,
            written_files,
        }
    }
}

impl ProjectFamilyMediaFileImporter for RuntimeProjectFamilyMediaFileImporter<'_> {
    fn write_imported_media_field(
        &mut self,
        old_asset: &media_asset::Model,
        new_asset: &media_asset::Model,
        field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        (|| -> Result<Option<String>, String> {
            let old_value = media_asset_path(old_asset, field)?;
            if old_value.is_none() {
                return Ok(None);
            }
            let entry = self
                .media_payload
                .entries
                .get(&media_file_key(old_asset.id, field))
                .ok_or_else(|| format!("媒体资产缺少文件条目: {} {}", old_asset.id, field))?;
            let bytes = self
                .media_payload
                .bytes
                .get(&entry.package_path)
                .ok_or_else(|| format!("媒体文件字节缺失: {}", entry.package_path))?;
            let extension = required_package_extension(&entry.package_path)?;
            let suffix = if field == MEDIA_FILE_FIELD_THUMBNAIL {
                "_thumbnail"
            } else {
                ""
            };
            let relative = format!(
                "projects/{}/media/{}/imported_package_{}{}.{}",
                new_asset.project_id, new_asset.media_kind, new_asset.id, suffix, extension
            );
            write_relative_data_file(self.data_dir, &relative, bytes, self.written_files)?;
            Ok(Some(relative))
        })()
        .map_err(ProjectFamilyMediaImportError::Write)
    }
}

fn write_relative_data_file(
    data_dir: &Path,
    relative: &str,
    bytes: &[u8],
    written_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let target = data_dir.join(relative);
    if target.exists() {
        return Err(format!("导入媒体目标文件已存在: {}", target.display()));
    }
    let parent = target
        .parent()
        .ok_or_else(|| format!("导入媒体目标路径缺少父目录: {}", target.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|err| format!("创建导入媒体目录失败: {}: {err}", parent.display()))?;
    std::fs::write(&target, bytes)
        .map_err(|err| format!("写入导入媒体文件失败: {}: {err}", target.display()))?;
    written_files.push(target);
    Ok(())
}
