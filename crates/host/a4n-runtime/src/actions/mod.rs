pub(crate) mod access;
pub mod admin_records;
pub(crate) mod agent_domains {
    pub(crate) mod canvas;
    pub(crate) mod content_optimization;
}
pub mod agent_runtime;
pub mod auth_records;
pub mod blueprint_records;
pub mod canvas_workflow;
pub mod chapter_content;
pub mod chapter_generation;
pub mod chapter_scene;
pub mod chapter_visual_plan_records;
pub mod character_ai;
pub mod character_conflict;
pub mod character_implicit_review;
pub mod character_records;
pub mod config_records;
mod dependency_conflict;
pub mod embedding_model;
pub mod export_files;
pub mod fact_feedback;
pub mod foreshadowing_records;
pub mod galgame_records;
pub mod generation_analytics;
pub mod generation_tasks;
pub mod image_generation;
pub mod image_records;
pub mod import_analysis;
pub mod import_file;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration_records;
pub mod logging;
pub mod manga_pipeline;
pub mod manga_records;
pub mod model_usage;
pub mod oauth;
pub mod object_profile_records;
pub mod optimize;
pub mod outline_records;
pub mod package;
pub mod portrait_generation;
pub mod portrait_records;
pub mod profile_materials;
pub mod project_collaboration;
pub mod project_duplication;
pub mod project_records;
pub mod project_status;
pub mod project_transfer;
pub mod prompt_records;
pub mod rag_search;
pub(crate) mod request_preview;
pub mod scene_profile_records;
pub mod scene_profile_sync;
pub mod settings_all;
pub mod shooting_script_records;
pub mod source_sync;
pub mod storage_paths;
pub mod task_records;
pub mod theme;
pub mod video_generation;
pub mod video_prompts;
pub mod video_records;

use a4n_error::AppError;

pub(crate) fn app_error_message(error: AppError) -> String {
    error.to_string()
}

pub(crate) async fn sidecar_root_project_id(
    state: &crate::AppState,
    project_id: i64,
) -> Result<Option<i64>, String> {
    if !state.runtime_profile().uses_sidecars() {
        return Ok(None);
    }
    state
        .project_access_application()
        .collaboration_root(project_id)
        .await
        .map(|root| Some(root.id))
        .map_err(app_error_message)
}
