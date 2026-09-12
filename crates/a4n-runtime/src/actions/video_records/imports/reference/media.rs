use super::super::contracts::{ImportedVideoSource, UploadReferenceMediaVideoInput};
use super::super::source::{
    persist_imported_video_file, probe_imported_video_properties, uploaded_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::GeneratedVideoSummary;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_records::{complete_video_record, video_record_output_path};
use chrono::Utc;

pub async fn upload_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state.temp_dir(),
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "导入参考媒体视频",
    )?;
    create_imported_reference_media_video(
        state,
        user_id,
        project_id,
        chapter_id,
        input.segment_plan_prompt_id,
        input.target_segment,
        source,
    )
    .await
}

async fn create_imported_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    segment_prompt_id: i64,
    target_segment: a4n_api_contract::video::VideoSegmentTarget,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let properties = probe_imported_video_properties(&source, "导入参考媒体视频")?;
    let imported_at = Utc::now().to_rfc3339();
    let record = state
        .video_import_application()
        .create_imported_reference_media_video_record(
            project_id,
            chapter_id,
            properties,
            &source.file_name,
            source.extension,
            &imported_at,
            segment_prompt_id,
            &target_segment.source_shot_id,
            target_segment.segment_index_in_shot,
        )
        .await
        .map_err(crate::actions::app_error_message)?;

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
                "reference_media_import",
                source.extension,
            )
        },
        "复制导入参考媒体视频失败",
        "导入参考媒体视频",
        "导入参考媒体视频文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state.data_dir(),
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        None,
        None,
        "导入参考媒体视频记录完成失败",
    )
    .await?;
    Ok(updated)
}
