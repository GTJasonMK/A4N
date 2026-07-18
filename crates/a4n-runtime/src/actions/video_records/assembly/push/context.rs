use super::super::cache::{
    assembled_video_output_path, assembly_cache_manifest_path, assembly_dimensions, assembly_fps,
    build_assembly_cache_manifest,
};
use super::super::clips::assembly_clips;
use super::super::contracts::AssemblyRequestContext;
use super::workflow::validate_assembly_workflow_mode;
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_prompts::resolve_project_chapter_video_source;

pub(in crate::actions::video_records::assembly) async fn assembly_request_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    action: &str,
) -> Result<AssemblyRequestContext, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, action, false).await?;
    let workflow_mode = validate_assembly_workflow_mode(&workflow_mode)?;
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let clips = assembly_clips(state, rows, project_id, chapter_id, workflow_mode)?;
    let (width, height) = assembly_dimensions(&clips);
    let fps = assembly_fps(&clips);
    let output_path = assembled_video_output_path(state, project_id, chapter_id, workflow_mode)?;
    let manifest_path = assembly_cache_manifest_path(&output_path);
    let manifest = build_assembly_cache_manifest(
        project_id,
        chapter_id,
        workflow_mode,
        width,
        height,
        fps,
        &clips,
    )?;
    Ok(AssemblyRequestContext {
        clips,
        output_path,
        manifest_path,
        manifest,
    })
}
