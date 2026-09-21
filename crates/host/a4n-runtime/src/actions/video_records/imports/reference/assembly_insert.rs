use super::super::contracts::{ImportedVideoSource, UploadAssemblyInsertVideoInput};
use super::super::source::{
    persist_imported_video_file, probe_imported_video_properties, uploaded_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::GeneratedVideoSummary;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_records::{complete_video_record, video_record_output_path};
use chrono::Utc;

pub async fn upload_assembly_insert_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: UploadAssemblyInsertVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state.temp_dir(),
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "插入视频",
    )?;
    create_assembly_insert_video(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input.insert_after_video_id,
        source,
    )
    .await
}

async fn create_assembly_insert_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    insert_after_video_id: Option<i64>,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let properties = probe_imported_video_properties(state, &source, "插入视频").await?;
    let imported_at = Utc::now().to_rfc3339();
    let record = state
        .video_assembly_import_application()
        .create_imported_assembly_insert_video_record(
            project_id,
            chapter_id,
            &workflow_mode,
            insert_after_video_id,
            properties,
            &source.file_name,
            source.extension,
            &imported_at,
        )
        .await
        .map_err(app_error_message)?;

    let (output, output_path_str) = persist_imported_video_file(
        state,
        state.data_dir(),
        record.id,
        &source.path,
        || {
            video_record_output_path(
                state.data_dir(),
                project_id,
                Some(chapter_id),
                record.id,
                "assembly_insert",
                source.extension,
            )
        },
        "复制插入视频失败",
        "插入视频",
        "插入视频文件",
    )
    .await?;
    let thumbnail_path =
        best_effort_video_thumbnail_path(state, project_id, chapter_id, record.id, &output).await;
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        None,
        None,
        "插入视频记录完成失败",
    )
    .await?;
    Ok(updated)
}
