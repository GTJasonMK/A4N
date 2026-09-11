use super::contracts::{ImportedVideoSource, UploadChapterVideoInput};
use super::source::{
    persist_imported_video_file, probe_imported_video_properties, uploaded_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::GeneratedVideoSummary;
use crate::actions::video_generation::media::{
    append_secondary_error, best_effort_video_thumbnail_path, capture_video_tail_frame_asset,
};
use crate::actions::video_records::{
    complete_video_record, delete_video_record_cleanup_error, video_record_output_path,
};
use a4n_orchestrator::video::{
    completed_provider_metadata, is_first_frame_chain_workflow,
    should_capture_video_tail_frame_fields,
};
use a4n_video_contract::VideoProviderResultTarget;
use chrono::Utc;
use serde_json::Value;

pub async fn upload_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state.temp_dir(),
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "导入视频",
    )?;
    create_imported_chapter_video(
        state,
        user_id,
        project_id,
        chapter_id,
        input.prompt_id,
        source,
    )
    .await
}

async fn create_imported_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let properties = probe_imported_video_properties(&source, "导入视频")?;
    let imported_at = Utc::now().to_rfc3339();
    let record = state
        .video_record_application()
        .create_imported_chapter_video_record(
            project_id,
            chapter_id,
            prompt_id,
            properties,
            &source.file_name,
            source.extension,
            &imported_at,
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
                "imported_video",
                source.extension,
            )
        },
        "复制导入视频失败",
        "导入视频",
        "导入视频文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state.data_dir(),
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame_fields(
        &record.video_type,
        record.workflow_mode.as_deref(),
    ) {
        let target = VideoProviderResultTarget {
            video_id: record.id,
            project_id: record.project_id,
            chapter_id: record.chapter_id,
            video_type: record.video_type.clone(),
            workflow_mode: record.workflow_mode.clone(),
        };
        match capture_video_tail_frame_asset(state, &target, chapter_id, &output).await {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                if is_first_frame_chain_workflow(record.workflow_mode.as_deref()) {
                    let cleanup_error =
                        crate::actions::video_generation::cleanup_saved_video_outputs(
                            state,
                            record.id,
                            &output,
                            thumbnail_path.as_deref(),
                        )
                        .await;
                    let delete_error = delete_video_record_cleanup_error(
                        state.video_record_application(),
                        record.id,
                    )
                    .await;
                    return Err(append_secondary_error(
                        append_secondary_error(
                            format!("导入视频已复制但首帧串联尾帧采集失败：{err}"),
                            cleanup_error,
                        ),
                        delete_error,
                    ));
                }
                tail_frame_capture_error = Some(err);
                None
            }
        }
    } else {
        None
    };
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        record.prompt_id,
        completed_provider_metadata(Value::Null, tail_frame_metadata, tail_frame_capture_error),
        "导入视频记录完成失败",
    )
    .await?;
    Ok(updated)
}
