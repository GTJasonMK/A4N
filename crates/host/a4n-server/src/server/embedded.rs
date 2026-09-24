use std::env;

use super::*;

#[cfg(feature = "test-embedded-processes")]
pub(crate) fn run_if_requested() -> anyhow::Result<Option<()>> {
    if env::var(CONTROL_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_control::run()?;
        return Ok(Some(()));
    }
    if env::var(EDGE_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_edge::run()?;
        return Ok(Some(()));
    }
    if env::var(ADMIN_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_admin_worker::run()?;
        return Ok(Some(()));
    }
    if env::var("A4N_AGENT_WORKER_PROCESS").as_deref() == Ok("1") {
        a4n_agent_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(ARTIFACT_MEDIA_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_artifact_media_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(OUTLINE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_outline_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(BLUEPRINT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_blueprint_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(ASSET_GRAPH_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_asset_graph_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(FORESHADOWING_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_foreshadowing_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(OBJECT_PROFILE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_object_profile_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(SCENE_PROFILE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_scene_profile_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROFILE_MATERIAL_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_profile_material_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROJECT_STATUS_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_project_status_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PORTRAIT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_portrait_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(SOURCE_SNAPSHOT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_source_snapshot_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(THEME_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_theme_read_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROJECT_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_project_read_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(IMPORT_REVIEW_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_import_review_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(INSPIRATION_HISTORY_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_inspiration_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROJECT_ACCESS_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_project_access_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROJECT_COLLABORATION_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_project_collaboration_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(PROJECT_MUTATION_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_project_mutation_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(CHAPTER_SCENE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_chapter_scene_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(CHAPTER_CONTENT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_chapter_content_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(WORKFLOW_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_workflow_read_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(WORKFLOW_LIFECYCLE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_workflow_lifecycle_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(CONFIG_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_config_read_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(CHARACTER_MUTATION_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_character_mutation_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(VIDEO_RECORD_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_video_record_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(MANGA_RECORD_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_manga_record_worker::run()?;
        return Ok(Some(()));
    }
    if env::var(CANVAS_RECORD_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
        a4n_canvas_record_worker::run()?;
        return Ok(Some(()));
    }

    Ok(None)
}
