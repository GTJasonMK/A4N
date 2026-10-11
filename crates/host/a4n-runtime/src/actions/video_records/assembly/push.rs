use super::cache::{AssemblyCacheManifestInput, build_assembly_cache_manifest};
use super::contracts::{AssemblyClip, AssemblyRequestContext};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;

pub(super) async fn assembly_request_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    action: &str,
) -> Result<Option<AssemblyRequestContext>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let prepared = state
        .video_assembly_planning_application()
        .assembly_records(project_id, chapter_id, &workflow_mode, action)
        .await
        .map_err(app_error_message)?;
    let clips = prepared
        .clips
        .into_iter()
        .map(|clip| {
            let path = a4n_storage::canonicalize_data_file_path(
                state.data_dir(),
                &clip.file_path,
                &format!("成片片段视频 #{}", clip.video_id),
            )?;
            Ok(AssemblyClip {
                video_id: clip.video_id,
                video_type: clip.video_type,
                shot_number: clip.shot_number,
                sort_order: clip.sort_order,
                insert_after_video_id: clip.insert_after_video_id,
                path,
                file_path: clip.file_path,
                duration_secs: clip.duration_secs,
                width: clip.width,
                height: clip.height,
                fps: clip.fps,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if clips.is_empty() {
        return Ok(None);
    }
    let (width, height, fps, duration_secs) =
        a4n_video::assembly_media_properties(clips.iter().map(|clip| {
            (
                clip.video_id,
                clip.duration_secs,
                clip.width,
                clip.height,
                clip.fps,
            )
        }))?;
    let chapter_videos_dir = crate::actions::video_generation::media::ensure_chapter_videos_dir(
        state.data_dir(),
        project_id,
        chapter_id,
    )?;
    let output_path = chapter_videos_dir.join(format!("assembled_{}.mp4", prepared.workflow_mode));
    let manifest_path = output_path.with_extension("mp4.cache.json");
    let manifest = build_assembly_cache_manifest(
        state,
        AssemblyCacheManifestInput {
            project_id,
            chapter_id,
            workflow_mode: &prepared.workflow_mode,
            width,
            height,
            fps,
            clips: &clips,
        },
    )
    .await?;
    Ok(Some(AssemblyRequestContext {
        clips,
        duration_secs,
        output_path,
        manifest_path,
        manifest,
    }))
}
