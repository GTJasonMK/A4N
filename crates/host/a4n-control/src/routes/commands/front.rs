mod auth;
mod config;
mod prompt;
mod rag_embedding;
mod theme;
mod workflow_task;

pub(super) use auth::{
    change_user_password, get_current_user, list_sessions, login_user, oauth2_authorize_url,
    oauth2_available_providers, oauth2_callback, refresh_token, register_user, revoke_session,
    update_user_profile,
};
pub(super) use config::{
    add_image_config, add_video_config, create_embedding_config, create_llm_config,
    delete_embedding_config, delete_llm_config, detect_image_config_models,
    detect_llm_config_models, detect_video_config_models, export_all_configs_json,
    import_all_configs, list_agent_skills, list_embedding_config, list_image_configs,
    list_llm_config, list_model_provider_catalog, list_video_configs,
    preview_test_embedding_config_request, preview_test_image_config_request,
    preview_test_llm_config_request, remove_image_config, remove_video_config,
    set_default_embedding_config, set_default_image_config, set_default_llm_config,
    set_default_video_config, test_embedding_config, test_image_config, test_llm_config,
    update_agent_skill_configuration, update_embedding_config, update_image_config,
    update_llm_config, update_video_config, web_fetch, web_search,
};
pub(super) use prompt::{
    prompt_list, prompt_list_categories, prompt_list_project_overrides, prompt_load_defaults,
    prompt_remove_project_override, prompt_render, prompt_render_ab, prompt_reset,
    prompt_set_project_override, prompt_update, prompt_validate_variables, prompt_version_diff,
    prompt_version_history,
};
pub(super) use rag_embedding::{
    cancel_embedding_model_download, delete_local_embedding_model, download_embedding_model,
    get_embedding_model_download_info, list_local_embedding_models, rag_enhanced_search,
    rag_enhanced_search_with_vectors, rag_ingest_all, rag_prepare_enhanced_search,
    rag_prepare_ingest_all, rag_upload_ingest_embeddings,
};
pub(super) use theme::{
    activate_theme, create_theme, delete_theme, delete_theme_resource, export_theme,
    export_theme_package, get_theme_preference, import_theme, import_theme_package, list_themes,
    update_theme, update_theme_preference, upload_theme_resource,
};
pub(super) use workflow_task::{
    cancel_generation_task, cancel_task, cancel_workflow, get_queue_stats, get_task_history,
    get_workflow_progress_events, get_workflow_run_detail, list_active_workflows,
    list_workflow_runs, resume_workflow_run,
};
