use super::super::super::assembly::{
    ASSEMBLY_INSERT_VIDEO_TYPE, assembly_clips, assembly_push_rows,
    ensure_insert_position_not_inside_pushed_prefix, next_assembly_insert_sort_order,
    validate_assembly_workflow_mode,
};
use super::super::contracts::{ImportedVideoSource, UploadAssemblyInsertVideoInput};
use super::super::source::{
    import_video_output_path, mark_imported_video_completed, persist_imported_video_file,
    probe_video_properties, uploaded_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
use a4n_orchestrator::video::CreateGeneratedVideoRecordDraftInput as CreateVideoRecord;
use chrono::Utc;
use serde_json::json;

pub async fn upload_assembly_insert_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: UploadAssemblyInsertVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
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
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入衔接片段", true)
        .await?;
    let workflow_mode = validate_assembly_workflow_mode(&workflow_mode)?;
    let properties = probe_video_properties(&source.path, "插入视频")?;
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let clips = assembly_clips(state, rows.clone(), project_id, chapter_id, workflow_mode)?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, insert_after_video_id)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&clips, insert_after_video_id)?;
    let imported_at = Utc::now().to_rfc3339();
    let anchor_label = insert_after_video_id
        .map(|id| format!("视频 #{id} 之后"))
        .unwrap_or_else(|| "成片开头".to_string());
    let metadata = json!({
        "shot_number": insert_after_shot_number.unwrap_or(0),
        "assembly_insert": {
            "sort_order": sort_order,
            "insert_after_video_id": insert_after_video_id,
            "insert_after_shot_number": insert_after_shot_number,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": imported_at,
        }
    });

    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_INSERT_VIDEO_TYPE.to_string(),
            workflow_mode: Some(workflow_mode.to_string()),
            prompt: format!("成片衔接片段，插入位置：{anchor_label}。"),
            negative_prompt: None,
            model: "external-assembly-insert".to_string(),
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
                "assembly_insert",
                source.extension,
            )
        },
        "复制插入视频失败",
        "插入视频",
        "插入视频文件",
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
        "插入视频记录完成失败",
        "插入视频文件",
        "插入视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
