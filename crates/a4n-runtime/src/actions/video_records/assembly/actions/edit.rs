use super::super::contracts::ApplyAssemblyPostprocessEditInput;
use super::super::postprocess::{
    MediaPostprocessEditInput, postprocess_edit_source, run_media_postprocess_edit,
};
use super::persistence::persist_assembly_record_output;
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_records::imports::{
    import_video_output_path, mark_imported_video_completed, probe_video_properties,
};
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::video::AssemblyPostprocessEditRecordInput;

pub async fn apply_assembly_postprocess_edit(
    state: &AppState,
    user_id: i64,
    push_video_id: i64,
    input: ApplyAssemblyPostprocessEditInput,
) -> Result<GeneratedVideoSummary, String> {
    let plan = state
        .video()
        .prepare_assembly_postprocess_edit_validated(push_video_id, &input.operation)
        .await
        .map_err(app_error_message)?;
    access::verify_project_editor_string(state, user_id, plan.push.project_id).await?;
    let chapter_id = plan
        .push
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", plan.push.id))?;
    let project_id = plan.push.project_id;
    let base_path = plan
        .base
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("后处理基底视频 #{} 没有本地文件", plan.base.id))?;
    let base_path = canonicalize_data_file_path(
        state.data_dir(),
        base_path,
        &format!("后处理基底视频 #{}", plan.base.id),
    )?;
    let source = postprocess_edit_source(state, &plan, &input).await?;
    let source_duration_secs = source
        .as_ref()
        .map(|source| probe_video_properties(&source.path, "后处理来源视频"))
        .transpose()?
        .map(|properties| properties.duration_secs);
    let source_video_id = source.as_ref().and_then(|source| source.source_video_id);
    let prepared = state
        .video()
        .create_assembly_postprocess_edit_record_validated(
            plan,
            AssemblyPostprocessEditRecordInput {
                at_secs: input.at_secs,
                start_secs: input.start_secs,
                end_secs: input.end_secs,
                source_video_id,
                source_file_name: source.as_ref().and_then(|source| source.file_name.clone()),
                source_duration_secs,
            },
        )
        .await
        .map_err(app_error_message)?;
    let record = prepared.video;
    let base = prepared.base;
    let operation = prepared.operation;
    let start_secs = prepared.start_secs;
    let end_secs = prepared.end_secs;
    let (output, output_path_str, size) = persist_assembly_record_output(
        state,
        record.id,
        || {
            import_video_output_path(
                state.data_dir(),
                project_id,
                chapter_id,
                record.id,
                "postprocess_edit",
                "mp4",
            )
        },
        |output| {
            run_media_postprocess_edit(MediaPostprocessEditInput {
                base_path: &base_path,
                source_path: source.as_ref().map(|source| source.path.as_path()),
                output_path: output,
                operation,
                start_secs,
                end_secs,
                base_duration_secs: base.duration_secs,
                width: base.width,
                height: base.height,
                fps: base.fps,
            })
        },
        "后处理编辑视频文件",
        "后处理编辑视频",
        "后处理编辑视频文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state.data_dir(),
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        size,
        None,
        None,
        "后处理编辑视频记录完成失败",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
