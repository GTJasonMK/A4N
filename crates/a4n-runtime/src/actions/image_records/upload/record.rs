use super::super::deletion::remove_image_delete_file;
use super::super::rules::append_secondary_error;
use super::{file::create_imported_image_output, metadata::imported_image_metadata};
use crate::{
    AppState,
    actions::image_generation::{GeneratedImageSummary, to_summary},
    file_utils::persist_created_media_file,
};
use a4n_orchestrator::video::CreateGeneratedImageRecordDraftInput as CreateImageRecord;
use serde_json::Value;
use std::io::Write;

pub(super) struct ImportedImageRecordInput {
    pub(super) image_type: String,
    pub(super) prompt: String,
    pub(super) model: String,
    pub(super) chapter_id: Option<i64>,
    pub(super) panel_id: Option<String>,
    pub(super) metadata_json: Option<Value>,
    pub(super) imported: super::ImportedImageFile,
}

pub(super) async fn create_imported_image_record(
    state: &AppState,
    project_id: i64,
    input: ImportedImageRecordInput,
) -> Result<GeneratedImageSummary, String> {
    let ImportedImageRecordInput {
        image_type,
        prompt,
        model,
        chapter_id,
        panel_id,
        metadata_json,
        imported,
    } = input;
    let (output_path, mut output_file) = create_imported_image_output(
        &state.config.data_dir,
        project_id,
        chapter_id,
        imported.extension,
    )?;
    let (relative_path, file_size) = persist_created_media_file(
        &state.config.data_dir,
        &output_path,
        move |_| {
            output_file
                .write_all(&imported.bytes)
                .map_err(|err| format!("写入导入图片失败：{err}"))
        },
        "导入图片文件",
        "导入图片",
        "导入图片文件",
    )?;
    let metadata_json = imported_image_metadata(
        metadata_json,
        imported.source_file_name,
        imported.extension,
        imported.mime_type,
    );
    let record = match state
        .orchestrator
        .create_generated_image_record(CreateImageRecord {
            project_id,
            image_type,
            file_path: relative_path,
            prompt,
            model,
            width: imported.width,
            height: imported.height,
            file_size: Some(file_size),
            chapter_id,
            panel_id,
            metadata_json,
            created_at: None,
        })
        .await
    {
        Ok(record) => record,
        Err(err) => {
            let cleanup_error = remove_image_delete_file(&output_path).err();
            return Err(append_secondary_error(
                format!("导入图片记录创建失败：{err}"),
                cleanup_error,
            ));
        }
    };
    Ok(to_summary(&record))
}
