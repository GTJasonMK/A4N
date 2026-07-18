use super::super::contracts::{ImportedVideoSource, UploadReferenceMediaVideoInput};
use super::super::source::{
    import_video_output_path, mark_imported_video_completed, persist_imported_video_file,
    probe_video_properties, uploaded_video_source, validate_reference_media_import_options,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
use a4n_orchestrator::video::CreateGeneratedVideoRecordDraftInput as CreateVideoRecord;
use chrono::Utc;
use serde_json::json;

pub async fn upload_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
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
        input.prompt,
        input.duration_secs,
        input.aspect_ratio,
        source,
    )
    .await
}

async fn create_imported_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt: String,
    duration_secs: i32,
    aspect_ratio: String,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入参考媒体视频", true)
        .await?;
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("参考媒体视频 prompt 不能为空".to_string());
    }
    let import_options = validate_reference_media_import_options(duration_secs, &aspect_ratio)?;
    let properties = probe_video_properties(&source.path, "导入参考媒体视频")?;
    let metadata = json!({
        "provider_backend": "external-import",
        "reference_media": {
            "functionMode": "omni_reference",
            "aspect_ratio": &import_options.aspect_ratio,
        },
        "import": {
            "external": true,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": Utc::now().to_rfc3339(),
        }
    });

    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "reference_media_video".into(),
            workflow_mode: Some("reference_media".to_string()),
            prompt: prompt.to_string(),
            negative_prompt: None,
            model: "external-reference-media-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(crate::actions::app_error_message)?;

    let (output, output_path_str) = persist_imported_video_file(
        state,
        record.id,
        &source.path,
        || {
            import_video_output_path(
                state,
                project_id,
                chapter_id,
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
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        None,
        None,
        None,
        "导入参考媒体视频记录完成失败",
        "导入参考媒体视频文件",
        "视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
