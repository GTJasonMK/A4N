mod admin;
mod characters;
mod front;
mod import_project;
mod media;

use self::characters::*;
use self::front::*;
use self::import_project::*;
use self::media::*;

use super::auth::{authenticated_user, issue_auth_response};
use super::error::ApiError;
use super::projects::project_summary_with_family_for_user;
use crate::state::ServerState;
use a4n_core::event::{
    channel::{TaskHandle, TaskProgress, TaskType},
    sse::{ProgressMeta, SseEvent},
};
use a4n_core::models::project::novel_project;
use a4n_runtime::actions::auth_records as auth_records_actions;
use a4n_runtime::actions::blueprint_records as blueprint_records_actions;
use a4n_runtime::actions::canvas_workflow as canvas_workflow_actions;
use a4n_runtime::actions::chapter_content as chapter_content_actions;
use a4n_runtime::actions::chapter_generation as chapter_generation_actions;
use a4n_runtime::actions::chapter_scene as chapter_scene_actions;
use a4n_runtime::actions::character_ai as character_ai_actions;
use a4n_runtime::actions::character_conflict as character_conflict_actions;
use a4n_runtime::actions::character_implicit_review as character_implicit_review_actions;
use a4n_runtime::actions::character_records as character_records_actions;
use a4n_runtime::actions::config_records as config_records_actions;
use a4n_runtime::actions::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::actions::embedding_model as embedding_model_actions;
use a4n_runtime::actions::export_files as export_files_actions;
use a4n_runtime::actions::galgame_records as galgame_records_actions;
use a4n_runtime::actions::generation_analytics as generation_analytics_actions;
use a4n_runtime::actions::image_generation as image_generation_actions;
use a4n_runtime::actions::image_records as image_records_actions;
use a4n_runtime::actions::import_analysis as import_analysis_actions;
use a4n_runtime::actions::import_reverse as import_reverse_actions;
use a4n_runtime::actions::import_review as import_review_actions;
use a4n_runtime::actions::inspiration_records as inspiration_records_actions;
use a4n_runtime::actions::logging as logging_actions;
use a4n_runtime::actions::manga_pipeline as manga_pipeline_actions;
use a4n_runtime::actions::manga_records as manga_records_actions;
use a4n_runtime::actions::oauth as oauth_actions;
use a4n_runtime::actions::object_profile_records as object_profile_records_actions;
use a4n_runtime::actions::optimize as optimize_actions;
use a4n_runtime::actions::outline_records as outline_records_actions;
use a4n_runtime::actions::portrait_generation as portrait_generation_actions;
use a4n_runtime::actions::portrait_records as portrait_records_actions;
use a4n_runtime::actions::profile_materials as profile_materials_actions;
use a4n_runtime::actions::project_collaboration as project_collaboration_actions;
use a4n_runtime::actions::project_records as project_records_actions;
use a4n_runtime::actions::project_status as project_status_actions;
use a4n_runtime::actions::project_transfer as project_transfer_actions;
use a4n_runtime::actions::prompt_records as prompt_records_actions;
use a4n_runtime::actions::rag_search as rag_search_actions;
use a4n_runtime::actions::scene_profile_records as scene_profile_records_actions;
use a4n_runtime::actions::scene_profile_sync as scene_profile_sync_actions;
use a4n_runtime::actions::storage_paths as storage_paths_actions;
use a4n_runtime::actions::task_records as task_records_actions;
use a4n_runtime::actions::theme as theme_actions;
use a4n_runtime::actions::video_generation as video_generation_actions;
use a4n_runtime::actions::video_prompts as video_prompt_actions;
use a4n_runtime::actions::video_records as video_records_actions;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;

pub async fn invoke(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(cmd): Path<String>,
    Json(args): Json<Value>,
) -> Result<Response, ApiError> {
    if cmd == "oauth2_authorize_url" {
        return Ok(Json(oauth2_authorize_url(&state, &args).await?).into_response());
    }
    if cmd == "oauth2_callback" {
        return Ok(Json(oauth2_callback(&state, &headers, &args).await?).into_response());
    }

    let user = authenticated_user(&state, &headers).await?;
    verify_user_arg(&args, user.id)?;
    let runtime_config_snapshot =
        a4n_runtime::runtime_config::snapshot_from_args(&args).map_err(ApiError::bad_request)?;

    if let Some(command) = reverse_engineer_web_command(&cmd) {
        let result = run_with_runtime_context(user.id, runtime_config_snapshot, async {
            run_reverse_engineer_web_command(&state, user.id, &args, command).await
        })
        .await?;
        return Ok(Json(result).into_response());
    }

    if is_canvas_web_command(&cmd) {
        let result = run_with_runtime_context(user.id, runtime_config_snapshot, async {
            run_canvas_web_command(&state, user.id, &args, &cmd).await
        })
        .await?;
        return Ok(Json(result).into_response());
    }

    let result = run_with_runtime_context(
        user.id,
        runtime_config_snapshot,
        Box::pin(async {
            let result = match cmd.as_str() {
                "get_user" => get_user(&state, user.id, &args).await?,
                "update_user_profile" => update_user_profile(&state, user.id, &args).await?,
                "change_user_password" => change_user_password(&state, user.id, &args).await?,
                "list_sessions" => list_sessions(&state, user.id, &args).await?,
                "revoke_session" => revoke_session(&state, user.id, &args).await?,
                "list_themes" => list_themes(&state, user.id, &args).await?,
                "list_user_themes" => list_user_themes(&state, user.id, &args).await?,
                "get_system_default_theme" => {
                    get_system_default_theme(&state, user.id, &args).await?
                }
                "create_theme" => create_theme(&state, user.id, &args).await?,
                "update_theme" => update_theme(&state, user.id, &args).await?,
                "delete_theme" => delete_theme(&state, user.id, &args).await?,
                "activate_theme" => activate_theme(&state, user.id, &args).await?,
                "export_theme" => export_theme(&state, user.id, &args).await?,
                "import_theme" => import_theme(&state, user.id, &args).await?,
                "export_all_configs_json" => {
                    export_all_configs_json(&state, user.id, &args).await?
                }
                "import_all_configs" => import_all_configs(&state, user.id, &args).await?,
                "get_theme_css_variables" => {
                    get_theme_css_variables(&state, user.id, &args).await?
                }
                "list_llm_config" => list_llm_config(&state, user.id, &args).await?,
                "create_llm_config" => create_llm_config(&state, user.id, &args).await?,
                "update_llm_config" => update_llm_config(&state, user.id, &args).await?,
                "set_default_llm_config" => set_default_llm_config(&state, user.id, &args).await?,
                "delete_llm_config" => delete_llm_config(&state, user.id, &args).await?,
                "get_llm_max_tokens_config" => {
                    get_llm_max_tokens_config(&state, user.id, &args).await?
                }
                "update_llm_max_tokens_config" => {
                    update_llm_max_tokens_config(&state, user.id, &args).await?
                }
                "list_embedding_config" => list_embedding_config(&state, user.id, &args).await?,
                "create_embedding_config" => {
                    create_embedding_config(&state, user.id, &args).await?
                }
                "update_embedding_config" => {
                    update_embedding_config(&state, user.id, &args).await?
                }
                "set_default_embedding_config" => {
                    set_default_embedding_config(&state, user.id, &args).await?
                }
                "delete_embedding_config" => {
                    delete_embedding_config(&state, user.id, &args).await?
                }
                "list_embedding_backends" => {
                    list_embedding_backends(&state, user.id, &args).await?
                }
                "rag_enhanced_search" => rag_enhanced_search(&state, user.id, &args).await?,
                "rag_prepare_enhanced_search" => {
                    rag_prepare_enhanced_search(&state, user.id, &args).await?
                }
                "rag_enhanced_search_with_vectors" => {
                    rag_enhanced_search_with_vectors(&state, user.id, &args).await?
                }
                "rag_ingest_all" => rag_ingest_all(&state, user.id, &args).await?,
                "rag_prepare_ingest_all" => rag_prepare_ingest_all(&state, user.id, &args).await?,
                "rag_upload_ingest_embeddings" => {
                    rag_upload_ingest_embeddings(&state, user.id, &args).await?
                }
                "test_embedding_config" => test_embedding_config(&state, user.id, &args).await?,
                "preview_test_embedding_config_request" => {
                    preview_test_embedding_config_request(&state, user.id, &args).await?
                }
                "list_local_embedding_models" => {
                    list_local_embedding_models(&state, user.id, &args).await?
                }
                "get_embedding_model_download_info" => {
                    get_embedding_model_download_info(&state, user.id, &args).await?
                }
                "download_embedding_model" => {
                    download_embedding_model(&state, user.id, &args).await?
                }
                "cancel_embedding_model_download" => {
                    cancel_embedding_model_download(&state, user.id, &args).await?
                }
                "delete_local_embedding_model" => {
                    delete_local_embedding_model(&state, user.id, &args).await?
                }
                "list_image_configs" => list_image_configs(&state, user.id, &args).await?,
                "add_image_config" => add_image_config(&state, user.id, &args).await?,
                "update_image_config" => update_image_config(&state, user.id, &args).await?,
                "remove_image_config" => remove_image_config(&state, user.id, &args).await?,
                "activate_image_config" => activate_image_config(&state, user.id, &args).await?,
                "test_image_config" => test_image_config(&state, user.id, &args).await?,
                "preview_test_image_config_request" => {
                    preview_test_image_config_request(&state, user.id, &args).await?
                }
                "detect_image_config_models" => {
                    detect_image_config_models(&state, user.id, &args).await?
                }
                "list_video_configs" => list_video_configs(&state, user.id, &args).await?,
                "add_video_config" => add_video_config(&state, user.id, &args).await?,
                "update_video_config" => update_video_config(&state, user.id, &args).await?,
                "remove_video_config" => remove_video_config(&state, user.id, &args).await?,
                "activate_video_config" => activate_video_config(&state, user.id, &args).await?,
                "test_video_config" => test_video_config(&state, user.id, &args).await?,
                "detect_video_config_models" => {
                    detect_video_config_models(&state, user.id, &args).await?
                }
                "test_llm_config" => test_llm_config(&state, user.id, &args).await?,
                "preview_test_llm_config_request" => {
                    preview_test_llm_config_request(&state, user.id, &args).await?
                }
                "detect_llm_config_models" => {
                    detect_llm_config_models(&state, user.id, &args).await?
                }
                "prompt_render" => prompt_render(&state, user.id, &args).await?,
                "prompt_list" => prompt_list(&state, user.id, &args).await?,
                "prompt_load_defaults" => prompt_load_defaults(&state, user.id, &args).await?,
                "prompt_update" => prompt_update(&state, user.id, &args).await?,
                "prompt_reset" => prompt_reset(&state, user.id, &args).await?,
                "prompt_version_history" => prompt_version_history(&state, user.id, &args).await?,
                "prompt_version_diff" => prompt_version_diff(&state, user.id, &args).await?,
                "prompt_validate_variables" => {
                    prompt_validate_variables(&state, user.id, &args).await?
                }
                "prompt_dependency_graph" => {
                    prompt_dependency_graph(&state, user.id, &args).await?
                }
                "prompt_render_ab" => prompt_render_ab(&state, user.id, &args).await?,
                "prompt_get_dependencies" => {
                    prompt_get_dependencies(&state, user.id, &args).await?
                }
                "prompt_get_reverse_dependencies" => {
                    prompt_get_reverse_dependencies(&state, user.id, &args).await?
                }
                "prompt_set_project_override" => {
                    prompt_set_project_override(&state, user.id, &args).await?
                }
                "prompt_remove_project_override" => {
                    prompt_remove_project_override(&state, user.id, &args).await?
                }
                "prompt_list_project_overrides" => {
                    prompt_list_project_overrides(&state, user.id, &args).await?
                }
                "prompt_list_categories" => prompt_list_categories(&state, user.id, &args).await?,
                "get_user_usage" => admin::get_user_usage(&state, user.id, &args).await?,
                "get_token_summary" => admin::get_token_summary(&state, user.id, &args).await?,
                "list_pending_tasks" => admin::list_pending_tasks(&state, user.id, &args).await?,
                "cancel_admin_task" => admin::cancel_admin_task(&state, user.id, &args).await?,
                "health_check" => admin::health_check(&state, user.id, &args).await?,
                "list_all_projects" => admin::list_all_projects(&state, user.id, &args).await?,
                "get_usage_trends" => admin::get_usage_trends(&state, user.id, &args).await?,
                "get_admin_overview" => admin::get_admin_overview(&state, user.id, &args).await?,
                "list_active_workflows" => list_active_workflows(&state, user.id, &args).await?,
                "list_workflow_runs" => list_workflow_runs(&state, user.id, &args).await?,
                "get_workflow_run_detail" => {
                    get_workflow_run_detail(&state, user.id, &args).await?
                }
                "resume_workflow_run" => resume_workflow_run(&state, user.id, &args).await?,
                "get_workflow_progress_events" => {
                    get_workflow_progress_events(&state, user.id, &args).await?
                }
                "cancel_workflow" => cancel_workflow(&state, user.id, &args).await?,
                "cancel_task" => cancel_task(&state, user.id, &args).await?,
                "get_queue_stats" => get_queue_stats(&state, user.id, &args).await?,
                "get_task_history" => get_task_history(&state, user.id, &args).await?,
                "generate_scene_image" => generate_scene_image(&state, user.id, &args).await?,
                "preview_scene_image_request" => {
                    preview_scene_image_request(&state, user.id, &args).await?
                }
                "generate_page_image" => generate_page_image(&state, user.id, &args).await?,
                "preview_page_image_request" => {
                    preview_page_image_request(&state, user.id, &args).await?
                }
                "batch_generate_images" => batch_generate_images(&state, user.id, &args).await?,
                "preview_batch_images_request" => {
                    preview_batch_images_request(&state, user.id, &args).await?
                }
                "list_images_by_project" => list_images_by_project(&state, user.id, &args).await?,
                "list_images_by_chapter" => list_images_by_chapter(&state, user.id, &args).await?,
                "list_video_reference_images" => {
                    list_video_reference_images(&state, user.id, &args).await?
                }
                "list_profile_materials" => list_profile_materials(&state, user.id, &args).await?,
                "set_profile_primary_material" => {
                    set_profile_primary_material(&state, user.id, &args).await?
                }
                "upload_profile_material_image" => {
                    upload_profile_material_image(&state, user.id, &args).await?
                }
                "toggle_image_favorite" => toggle_image_favorite(&state, user.id, &args).await?,
                "update_image" => update_image(&state, user.id, &args).await?,
                "upload_image" => upload_image(&state, user.id, &args).await?,
                "delete_image" => delete_image(&state, user.id, &args).await?,
                "score_image" => score_image(&state, user.id, &args).await?,
                "list_videos_by_chapter" => list_videos_by_chapter(&state, user.id, &args).await?,
                "toggle_video_favorite" => toggle_video_favorite(&state, user.id, &args).await?,
                "update_video" => update_video(&state, user.id, &args).await?,
                "delete_video" => delete_video(&state, user.id, &args).await?,
                "delete_chapter_video_assets" => {
                    delete_chapter_video_assets(&state, user.id, &args).await?
                }
                "load_video_playback_data" => {
                    load_video_playback_data(&state, user.id, &args).await?
                }
                "assemble_chapter_videos" => {
                    assemble_chapter_videos(&state, user.id, &args).await?
                }
                "load_cached_assembled_chapter_video_preview" => {
                    load_cached_assembled_chapter_video_preview(&state, user.id, &args).await?
                }
                "push_assembled_chapter_video" => {
                    push_assembled_chapter_video(&state, user.id, &args).await?
                }
                "update_assembly_push_status" => {
                    update_assembly_push_status(&state, user.id, &args).await?
                }
                "apply_assembly_postprocess_edit" => {
                    apply_assembly_postprocess_edit(&state, user.id, &args).await?
                }
                "upload_chapter_video" => upload_chapter_video(&state, user.id, &args).await?,
                "upload_reference_media_video" => {
                    upload_reference_media_video(&state, user.id, &args).await?
                }
                "upload_assembly_insert_video" => {
                    upload_assembly_insert_video(&state, user.id, &args).await?
                }
                "trim_video" => trim_video(&state, user.id, &args).await?,
                "list_chapter_video_prompts" => {
                    list_chapter_video_prompts(&state, user.id, &args).await?
                }
                "generate_chapter_video_shot_analysis" => {
                    generate_chapter_video_shot_analysis(&state, user.id, &args).await?
                }
                "generate_chapter_video_segment_plan" => {
                    generate_chapter_video_segment_plan(&state, user.id, &args).await?
                }
                "generate_chapter_video_frame_prompts" => {
                    generate_chapter_video_frame_prompts(&state, user.id, &args).await?
                }
                "generate_chapter_video_prompts" => {
                    generate_chapter_video_prompts(&state, user.id, &args).await?
                }
                "preview_chapter_video_frame_prompt_requests" => {
                    preview_chapter_video_frame_prompt_requests(&state, user.id, &args).await?
                }
                "preview_chapter_video_prompt_requests" => {
                    preview_chapter_video_prompt_requests(&state, user.id, &args).await?
                }
                "update_chapter_video_prompt" => {
                    update_chapter_video_prompt(&state, user.id, &args).await?
                }
                "optimize_chapter_video_prompt" => {
                    optimize_chapter_video_prompt(&state, user.id, &args).await?
                }
                "delete_chapter_video_prompt" => {
                    delete_chapter_video_prompt(&state, user.id, &args).await?
                }
                "delete_chapter_video_prompt_group" => {
                    delete_chapter_video_prompt_group(&state, user.id, &args).await?
                }
                "generate_chapter_video" => generate_chapter_video(&state, user.id, &args).await?,
                "generate_chapter_videos_batch" => {
                    generate_chapter_videos_batch(&state, user.id, &args).await?
                }
                "poll_video_job" => poll_video_job(&state, user.id, &args).await?,
                "generate_reference_media_video" => {
                    generate_reference_media_video(&state, user.id, &args).await?
                }
                "preview_chapter_video_request" => {
                    preview_chapter_video_request(&state, user.id, &args).await?
                }
                "preview_video_provider_request_by_video_id" => {
                    preview_video_provider_request_by_video_id(&state, user.id, &args).await?
                }
                "preview_reference_media_video_request" => {
                    preview_reference_media_video_request(&state, user.id, &args).await?
                }
                "list_manga_panels" => list_manga_panels(&state, user.id, &args).await?,
                "create_manga_prompt" => create_manga_prompt(&state, user.id, &args).await?,
                "bind_manga_reference_image" => {
                    bind_manga_reference_image(&state, user.id, &args).await?
                }
                "update_manga_prompt" => update_manga_prompt(&state, user.id, &args).await?,
                "optimize_manga_prompt" => optimize_manga_prompt(&state, user.id, &args).await?,
                "delete_manga_prompt" => delete_manga_prompt(&state, user.id, &args).await?,
                "delete_manga_by_chapter" => {
                    delete_manga_by_chapter(&state, user.id, &args).await?
                }
                "get_manga_pipeline_status" => {
                    get_manga_pipeline_status(&state, user.id, &args).await?
                }
                "cancel_manga_pipeline" => cancel_manga_pipeline(&state, user.id, &args).await?,
                "start_manga_pipeline" => start_manga_pipeline(&state, user.id, &args).await?,
                "resume_manga_pipeline" => resume_manga_pipeline(&state, user.id, &args).await?,
                "preview_start_manga_pipeline_request" => {
                    preview_start_manga_pipeline_request(&state, user.id, &args).await?
                }
                "preview_resume_manga_pipeline_request" => {
                    preview_resume_manga_pipeline_request(&state, user.id, &args).await?
                }
                "get_galgame_script" => get_galgame_script(&state, user.id, &args).await?,
                "start_galgame_pipeline" => start_galgame_pipeline(&state, user.id, &args).await?,
                "cancel_galgame_pipeline" => {
                    cancel_galgame_pipeline(&state, user.id, &args).await?
                }
                "preview_galgame_pipeline_request" => {
                    preview_galgame_pipeline_request(&state, user.id, &args).await?
                }
                "get_galgame_pipeline_status" => {
                    get_galgame_pipeline_status(&state, user.id, &args).await?
                }
                "update_galgame_line" => update_galgame_line(&state, user.id, &args).await?,
                "insert_galgame_line" => insert_galgame_line(&state, user.id, &args).await?,
                "update_galgame_scene" => update_galgame_scene(&state, user.id, &args).await?,
                "delete_galgame_line" => delete_galgame_line(&state, user.id, &args).await?,
                "reorder_galgame_lines" => reorder_galgame_lines(&state, user.id, &args).await?,
                "update_galgame_choices" => update_galgame_choices(&state, user.id, &args).await?,
                "plan_galgame_assets" => plan_galgame_assets(&state, user.id, &args).await?,
                "run_galgame_asset_generation" => {
                    run_galgame_asset_generation(&state, user.id, &args).await?
                }
                "bind_galgame_media" => bind_galgame_media(&state, user.id, &args).await?,
                "unbind_galgame_media" => unbind_galgame_media(&state, user.id, &args).await?,
                "import_galgame_audio" => import_galgame_audio(&state, user.id, &args).await?,
                "export_galgame_webgal" => export_galgame_webgal(&state, user.id, &args).await?,
                "list_object_profiles" => list_object_profiles(&state, user.id, &args).await?,
                "create_object_profile" => create_object_profile(&state, user.id, &args).await?,
                "update_object_profile" => update_object_profile(&state, user.id, &args).await?,
                "delete_object_profile" => delete_object_profile(&state, user.id, &args).await?,
                "generate_object_profile_image_prompt" => {
                    generate_object_profile_image_prompt(&state, user.id, &args).await?
                }
                "generate_object_profile_image" => {
                    generate_object_profile_image(&state, user.id, &args).await?
                }
                "preview_object_profile_image_request" => {
                    preview_object_profile_image_request(&state, user.id, &args).await?
                }
                "list_scene_profiles" => list_scene_profiles(&state, user.id, &args).await?,
                "create_scene_profile" => create_scene_profile(&state, user.id, &args).await?,
                "update_scene_profile" => update_scene_profile(&state, user.id, &args).await?,
                "delete_scene_profile" => delete_scene_profile(&state, user.id, &args).await?,
                "generate_scene_profile_image_prompt" => {
                    generate_scene_profile_image_prompt(&state, user.id, &args).await?
                }
                "generate_scene_profile_image" => {
                    generate_scene_profile_image(&state, user.id, &args).await?
                }
                "sync_scene_profiles" => sync_scene_profiles(&state, user.id, &args).await?,
                "list_profiles" => list_profiles(&state, user.id, &args).await?,
                "create_profile" => create_profile(&state, user.id, &args).await?,
                "delete_profile" => delete_profile(&state, user.id, &args).await?,
                "add_profile_attribute" => add_profile_attribute(&state, user.id, &args).await?,
                "modify_profile_attribute" => {
                    modify_profile_attribute(&state, user.id, &args).await?
                }
                "attribute_history" => attribute_history(&state, user.id, &args).await?,
                "get_character_arc_status" => {
                    get_character_arc_status(&state, user.id, &args).await?
                }
                "get_character_voice_fingerprint" => {
                    get_character_voice_fingerprint(&state, user.id, &args).await?
                }
                "discover_character_profiles" => {
                    discover_character_profiles(&state, user.id, &args).await?
                }
                "preview_discover_character_profiles_request" => {
                    preview_discover_character_profiles_request(&state, user.id, &args).await?
                }
                "sync_all_character_profiles" => {
                    sync_all_character_profiles(&state, user.id, &args).await?
                }
                "preview_sync_all_character_profiles_request" => {
                    preview_sync_all_character_profiles_request(&state, user.id, &args).await?
                }
                "sync_character_from_chapters" => {
                    sync_character_from_chapters(&state, user.id, &args).await?
                }
                "preview_sync_character_from_chapters_request" => {
                    preview_sync_character_from_chapters_request(&state, user.id, &args).await?
                }
                "check_character_conflict" => {
                    check_character_conflict(&state, user.id, &args).await?
                }
                "preview_check_character_conflict_request" => {
                    preview_check_character_conflict_request(&state, user.id, &args).await?
                }
                "list_optimize_sessions" => list_optimize_sessions(&state, user.id, &args).await?,
                "get_optimize_session" => get_optimize_session(&state, user.id, &args).await?,
                "list_optimize_turns" => list_optimize_turns(&state, user.id, &args).await?,
                "list_optimize_patch_sets" => {
                    list_optimize_patch_sets(&state, user.id, &args).await?
                }
                "reject_pending_optimize_patch_sets" => {
                    reject_pending_optimize_patch_sets(&state, user.id, &args).await?
                }
                "delete_optimize_session" => {
                    delete_optimize_session(&state, user.id, &args).await?
                }
                "record_optimize_agent_event" => {
                    record_optimize_agent_event(&state, user.id, &args).await?
                }
                "create_optimization_chat_session" => {
                    create_optimization_chat_session(&state, user.id, &args).await?
                }
                "create_optimization_analysis_session" => {
                    create_optimization_analysis_session(&state, user.id, &args).await?
                }
                "prepare_optimization_analysis_session" => {
                    prepare_optimization_analysis_session(&state, user.id, &args).await?
                }
                "execute_optimization_analysis_session" => {
                    execute_optimization_analysis_session(&state, user.id, &args).await?
                }
                "preview_create_optimization_analysis_session_request" => {
                    preview_create_optimization_analysis_session_request(&state, user.id, &args)
                        .await?
                }
                "preview_prepare_optimization_analysis_session_request" => {
                    preview_prepare_optimization_analysis_session_request(&state, user.id, &args)
                        .await?
                }
                "propose_optimization_patches" => {
                    propose_optimization_patches(&state, user.id, &args).await?
                }
                "preview_propose_optimization_patches_request" => {
                    preview_propose_optimization_patches_request(&state, user.id, &args).await?
                }
                "apply_optimization_patches" => {
                    apply_optimization_patches(&state, user.id, &args).await?
                }
                "send_optimization_message" => {
                    send_optimization_message(&state, user.id, &args).await?
                }
                "preview_send_optimization_message_request" => {
                    preview_send_optimization_message_request(&state, user.id, &args).await?
                }
                "get_inspiration_history" => {
                    get_inspiration_history(&state, user.id, &args).await?
                }
                "converse_inspiration" => converse_inspiration(&state, user.id, &args).await?,
                "converse_inspiration_stream" => {
                    converse_inspiration_stream(&state, user.id, &args).await?
                }
                "preview_inspiration_request" => {
                    preview_inspiration_request(&state, user.id, &args).await?
                }
                "import_uploaded_file" => import_uploaded_file(&state, user.id, &args).await?,
                "list_import_queue" => list_import_queue(&state, user.id, &args).await?,
                "start_import_analysis" => start_import_analysis(&state, user.id, &args).await?,
                "update_import_status" => update_import_status(&state, user.id, &args).await?,
                "delete_import_review" => delete_import_review(&state, user.id, &args).await?,
                "get_import_review" => get_import_review(&state, user.id, &args).await?,
                "accept_import_review" => accept_import_review(&state, user.id, &args).await?,
                "modify_import_review" => modify_import_review(&state, user.id, &args).await?,
                "reject_import_review" => reject_import_review(&state, user.id, &args).await?,
                "batch_accept_reviews" => batch_accept_reviews(&state, user.id, &args).await?,
                "get_import_analysis_status" => {
                    get_import_analysis_status(&state, user.id, &args).await?
                }
                "get_project_import_analysis_progress" => {
                    get_project_import_analysis_progress(&state, user.id, &args).await?
                }
                "run_import_project_analysis" => {
                    run_import_project_analysis(&state, user.id, &args).await?
                }
                "preview_import_project_analysis_request" => {
                    preview_import_project_analysis_request(&state, user.id, &args).await?
                }
                "cancel_import_analysis" => cancel_import_analysis(&state, user.id, &args).await?,
                "check_import_dedup" => check_import_dedup(&state, user.id, &args).await?,
                "export_txt_data" => export_txt_data(&state, user.id, &args).await?,
                "export_markdown_data" => export_markdown_data(&state, user.id, &args).await?,
                "export_epub_data" => export_epub_data(&state, user.id, &args).await?,
                "export_pdf_data" => export_pdf_data(&state, user.id, &args).await?,
                "export_batch_data" => export_batch_data(&state, user.id, &args).await?,
                "export_outline_json_data" => {
                    export_outline_json_data(&state, user.id, &args).await?
                }
                "export_characters_json_data" => {
                    export_characters_json_data(&state, user.id, &args).await?
                }
                "list_portraits_by_project" => {
                    list_portraits_by_project(&state, user.id, &args).await?
                }
                "get_primary_portrait" => get_primary_portrait(&state, user.id, &args).await?,
                "set_primary_portrait" => set_primary_portrait(&state, user.id, &args).await?,
                "upload_portrait" => upload_portrait(&state, user.id, &args).await?,
                "generate_portrait_prompt" => {
                    generate_portrait_prompt(&state, user.id, &args).await?
                }
                "preview_generate_portrait_prompt_request" => {
                    preview_generate_portrait_prompt_request(&state, user.id, &args).await?
                }
                "generate_portrait" => generate_portrait(&state, user.id, &args).await?,
                "preview_generate_portrait_request" => {
                    preview_generate_portrait_request(&state, user.id, &args).await?
                }
                "regenerate_portrait" => regenerate_portrait(&state, user.id, &args).await?,
                "preview_regenerate_portrait_request" => {
                    preview_regenerate_portrait_request(&state, user.id, &args).await?
                }
                "generate_age_variant" => generate_age_variant(&state, user.id, &args).await?,
                "preview_generate_age_variant_request" => {
                    preview_generate_age_variant_request(&state, user.id, &args).await?
                }
                "generate_missing_portraits" => {
                    generate_missing_portraits(&state, user.id, &args).await?
                }
                "preview_generate_missing_portraits_request" => {
                    preview_generate_missing_portraits_request(&state, user.id, &args).await?
                }
                "list_snapshots" => list_snapshots(&state, user.id, &args).await?,
                "get_profile_state_at_chapter" => {
                    get_profile_state_at_chapter(&state, user.id, &args).await?
                }
                "get_snapshot" => get_snapshot(&state, user.id, &args).await?,
                "diff_profile_chapters" => diff_profile_chapters(&state, user.id, &args).await?,
                "check_profile_sync_conflict" => {
                    check_profile_sync_conflict(&state, user.id, &args).await?
                }
                "rollback_to_chapter" => rollback_to_chapter(&state, user.id, &args).await?,
                "list_behaviors" => list_behaviors(&state, user.id, &args).await?,
                "get_implicit_review" => get_implicit_review(&state, user.id, &args).await?,
                "check_implicit_update" => check_implicit_update(&state, user.id, &args).await?,
                "preview_check_implicit_update_request" => {
                    preview_check_implicit_update_request(&state, user.id, &args).await?
                }
                "apply_implicit_update_decision" => {
                    apply_implicit_update_decision(&state, user.id, &args).await?
                }
                "list_deletion_marks" => list_deletion_marks(&state, user.id, &args).await?,
                "request_attribute_deletion" => {
                    request_attribute_deletion(&state, user.id, &args).await?
                }
                "execute_attribute_deletion" => {
                    execute_attribute_deletion(&state, user.id, &args).await?
                }
                "reset_deletion_marks" => reset_deletion_marks(&state, user.id, &args).await?,
                "get_storage_paths" => get_storage_paths(&state, user.id, &args).await?,
                "list_orphan_project_data_dirs" => {
                    list_orphan_project_data_dirs(&state, user.id, &args).await?
                }
                "cleanup_orphan_project_data_dirs" => {
                    cleanup_orphan_project_data_dirs(&state, user.id, &args).await?
                }
                "get_log_dir" => get_log_dir(&state, user.id, &args).await?,
                "read_recent_logs" => read_recent_logs(&state, user.id, &args).await?,
                "set_log_level" => set_log_level(&state, user.id, &args).await?,
                "get_generation_call_analytics" => {
                    get_generation_call_analytics(&state, user.id, &args).await?
                }
                "update_project" => update_project(&state, user.id, &args).await?,
                "get_project_asset_dependency_graph" => {
                    get_project_asset_dependency_graph(&state, user.id, &args).await?
                }
                "get_project_asset_dependency_graph_node_detail" => {
                    get_project_asset_dependency_graph_node_detail(&state, user.id, &args).await?
                }
                "delete_project" => delete_project(&state, user.id, &args).await?,
                "reset_project_generated_data" => {
                    reset_project_generated_data(&state, user.id, &args).await?
                }
                "list_project_members" => list_project_members(&state, user.id, &args).await?,
                "invite_project_member" => invite_project_member(&state, user.id, &args).await?,
                "list_project_invites" => list_project_invites(&state, user.id, &args).await?,
                "list_my_project_invites" => {
                    list_my_project_invites(&state, user.id, &args).await?
                }
                "accept_project_invite" => accept_project_invite(&state, user.id, &args).await?,
                "decline_project_invite" => decline_project_invite(&state, user.id, &args).await?,
                "revoke_project_invite" => revoke_project_invite(&state, user.id, &args).await?,
                "update_project_member_role" => {
                    update_project_member_role(&state, user.id, &args).await?
                }
                "remove_project_member" => remove_project_member(&state, user.id, &args).await?,
                "leave_project" => leave_project(&state, user.id, &args).await?,
                "import_project_full_upload" => {
                    import_project_full_upload(&state, user.id, &args).await?
                }
                "export_project_family_data" => {
                    export_project_family_data(&state, user.id, &args).await?
                }
                "export_derived_project_data" => {
                    export_derived_project_data(&state, user.id, &args).await?
                }
                "import_project_family_upload" => {
                    import_project_family_upload(&state, user.id, &args).await?
                }
                "export_video_studio_project_data" => {
                    export_video_studio_project_data(&state, user.id, &args).await?
                }
                "import_video_studio_project_upload" => {
                    import_video_studio_project_upload(&state, user.id, &args).await?
                }
                "forward_project_status" => forward_project_status(&state, user.id, &args).await?,
                "duplicate_project" => duplicate_project(&state, user.id, &args).await?,
                "branch_project" => branch_project(&state, user.id, &args).await?,
                "rollback_project_status" => {
                    rollback_project_status(&state, user.id, &args).await?
                }
                "preview_rollback" => preview_rollback(&state, user.id, &args).await?,
                "get_project_status_history" => {
                    get_project_status_history(&state, user.id, &args).await?
                }
                "list_chapters" => list_chapters(&state, user.id, &args).await?,
                "generate_chapter_stream" => {
                    generate_chapter_stream(&state, user.id, &args).await?
                }
                "prepare_chapter_generation_rag_embedding" => {
                    prepare_chapter_generation_rag_embedding(&state, user.id, &args).await?
                }
                "finish_chapter_generation_rag_context" => {
                    finish_chapter_generation_rag_context(&state, user.id, &args).await?
                }
                "preview_chapter_prompt" => preview_chapter_prompt(&state, user.id, &args).await?,
                "import_chapter" => import_chapter(&state, user.id, &args).await?,
                "update_chapter_content" => update_chapter_content(&state, user.id, &args).await?,
                "generate_chapter_analysis" => {
                    generate_chapter_analysis(&state, user.id, &args).await?
                }
                "preview_chapter_analysis_request" => {
                    preview_chapter_analysis_request(&state, user.id, &args).await?
                }
                "resume_chapter_analysis_sync" => {
                    resume_chapter_analysis_sync(&state, user.id, &args).await?
                }
                "preview_resume_chapter_analysis_sync_request" => {
                    preview_resume_chapter_analysis_sync_request(&state, user.id, &args).await?
                }
                "list_chapter_scenes" => list_chapter_scenes(&state, user.id, &args).await?,
                "update_chapter_scene" => update_chapter_scene(&state, user.id, &args).await?,
                "delete_chapter_scene" => delete_chapter_scene(&state, user.id, &args).await?,
                "reorder_chapter_scenes" => reorder_chapter_scenes(&state, user.id, &args).await?,
                "generate_chapter_scene" => generate_chapter_scene(&state, user.id, &args).await?,
                "preview_chapter_scene_request" => {
                    preview_chapter_scene_request(&state, user.id, &args).await?
                }
                "get_blueprint" => get_blueprint(&state, user.id, &args).await?,
                "generate_blueprint" => generate_blueprint(&state, user.id, &args).await?,
                "preview_generate_blueprint_request" => {
                    preview_generate_blueprint_request(&state, user.id, &args).await?
                }
                "refine_blueprint" => refine_blueprint(&state, user.id, &args).await?,
                "preview_refine_blueprint_request" => {
                    preview_refine_blueprint_request(&state, user.id, &args).await?
                }
                "list_blueprint_versions" => {
                    list_blueprint_versions(&state, user.id, &args).await?
                }
                "diff_blueprint_versions" => {
                    diff_blueprint_versions(&state, user.id, &args).await?
                }
                "restore_blueprint_version" => {
                    restore_blueprint_version(&state, user.id, &args).await?
                }
                "update_world_setting" => update_world_setting(&state, user.id, &args).await?,
                "list_parts" => list_parts(&state, user.id, &args).await?,
                "generate_parts" => generate_parts(&state, user.id, &args).await?,
                "preview_generate_parts_request" => {
                    preview_generate_parts_request(&state, user.id, &args).await?
                }
                "delete_latest_parts" => delete_latest_parts(&state, user.id, &args).await?,
                "regenerate_parts" => regenerate_parts(&state, user.id, &args).await?,
                "preview_regenerate_parts_request" => {
                    preview_regenerate_parts_request(&state, user.id, &args).await?
                }
                "regenerate_last_part" => regenerate_last_part(&state, user.id, &args).await?,
                "preview_regenerate_last_part_request" => {
                    preview_regenerate_last_part_request(&state, user.id, &args).await?
                }
                "regenerate_part_outline" => {
                    regenerate_part_outline(&state, user.id, &args).await?
                }
                "preview_regenerate_part_outline_request" => {
                    preview_regenerate_part_outline_request(&state, user.id, &args).await?
                }
                "continue_part_outlines" => continue_part_outlines(&state, user.id, &args).await?,
                "preview_continue_part_outlines_request" => {
                    preview_continue_part_outlines_request(&state, user.id, &args).await?
                }
                "get_generation_progress" => {
                    get_generation_progress(&state, user.id, &args).await?
                }
                "list_chapter_outlines" => list_chapter_outlines(&state, user.id, &args).await?,
                "delete_latest_chapter_outlines" => {
                    delete_latest_chapter_outlines(&state, user.id, &args).await?
                }
                "generate_part_chapter_outlines" => {
                    generate_part_chapter_outlines(&state, user.id, &args).await?
                }
                "preview_generate_part_chapter_outlines_request" => {
                    preview_generate_part_chapter_outlines_request(&state, user.id, &args).await?
                }
                "batch_generate_part_outlines" => {
                    batch_generate_part_outlines(&state, user.id, &args).await?
                }
                "preview_batch_generate_part_outlines_request" => {
                    preview_batch_generate_part_outlines_request(&state, user.id, &args).await?
                }
                "apply_beat_mapping" => apply_beat_mapping(&state, user.id, &args).await?,
                "preview_apply_beat_mapping_request" => {
                    preview_apply_beat_mapping_request(&state, user.id, &args).await?
                }
                "generate_chapter_outlines_by_count" => {
                    generate_chapter_outlines_by_count(&state, user.id, &args).await?
                }
                "preview_generate_chapter_outlines_by_count_request" => {
                    preview_generate_chapter_outlines_by_count_request(&state, user.id, &args)
                        .await?
                }
                "regenerate_chapter_outline" => {
                    regenerate_chapter_outline(&state, user.id, &args).await?
                }
                "preview_regenerate_chapter_outline_request" => {
                    preview_regenerate_chapter_outline_request(&state, user.id, &args).await?
                }
                "update_chapter_outline" => update_chapter_outline(&state, user.id, &args).await?,
                "get_chapter_outlines_pacing" => {
                    get_chapter_outlines_pacing(&state, user.id, &args).await?
                }
                other => {
                    return Err(ApiError::bad_request(format!(
                        "Web command 尚未实现: {other}"
                    )));
                }
            };
            Ok::<Value, ApiError>(result)
        }),
    )
    .await?;

    Ok(Json(result).into_response())
}

async fn run_with_runtime_context<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::runtime_config::RuntimeConfigSnapshot>,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>>,
{
    a4n_runtime::runtime_config::request_scope(user_id, snapshot, future).await
}

fn safe_workflow_task_id(
    provided: Option<String>,
    default_id: impl FnOnce() -> String,
) -> Result<String, ApiError> {
    let Some(candidate) = provided.map(|id| id.trim().to_string()) else {
        return Ok(default_id());
    };
    if candidate.is_empty() {
        return Err(ApiError::bad_request("task_id 不能为空"));
    }
    if candidate.len() > 160 {
        return Err(ApiError::bad_request("task_id 长度不能超过 160 个字符"));
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(ApiError::bad_request(
            "task_id 只能包含 ASCII 字母、数字、-、_、:、.",
        ));
    }
    Ok(candidate)
}

async fn require_admin_caller(
    _state: &ServerState,
    authenticated_user_id: i64,
    args: &Value,
) -> Result<(), ApiError> {
    let caller_user_id = i64_arg(args, "callerUserId")?;
    if caller_user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}

async fn emit_workflow_progress(
    state: &ServerState,
    task_id: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    emit_workflow_event(state, task_id, SseEvent::progress(current, total, message)).await;
}

async fn emit_workflow_progress_with_meta(
    state: &ServerState,
    task_id: &str,
    current: u32,
    total: u32,
    message: &str,
    meta: Option<ProgressMeta>,
) {
    let event = match meta {
        Some(meta) => SseEvent::progress_with_meta(current, total, message, meta),
        None => SseEvent::progress(current, total, message),
    };
    emit_workflow_event(state, task_id, event).await;
}

async fn emit_workflow_progress_bytes(
    state: &ServerState,
    task_id: &str,
    current: u64,
    total: u64,
    message: &str,
) {
    emit_workflow_event(
        state,
        task_id,
        SseEvent::progress_bytes(current, total, message),
    )
    .await;
}

async fn emit_workflow_chunk(state: &ServerState, task_id: &str, content: &str, index: u32) {
    emit_workflow_event(state, task_id, SseEvent::chunk(content, index)).await;
}

async fn emit_workflow_complete(state: &ServerState, task_id: &str, summary: Option<String>) {
    emit_workflow_event(state, task_id, SseEvent::complete(summary)).await;
}

async fn emit_workflow_error(
    state: &ServerState,
    task_id: &str,
    code: &str,
    message: &str,
    recoverable: bool,
) {
    emit_workflow_event(state, task_id, SseEvent::error(code, message, recoverable)).await;
}

async fn emit_workflow_event(state: &ServerState, task_id: &str, event: SseEvent) {
    let channel = SseEvent::workflow_channel(task_id);
    if let Ok(payload) = serde_json::to_value(event) {
        state.web_events.publish(&channel, payload).await;
    }
}

fn verify_user_arg(args: &Value, authenticated_user_id: i64) -> Result<(), ApiError> {
    let Some(value) = args.get("userId") else {
        return Ok(());
    };
    let Some(user_id) = value.as_i64() else {
        return Err(ApiError::bad_request("userId 必须是整数"));
    };
    if user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}

fn required_arg<'a>(args: &'a Value, key: &str) -> Result<&'a Value, ApiError> {
    args.get(key)
        .ok_or_else(|| ApiError::bad_request(format!("缺少参数: {key}")))
}

fn i64_arg(args: &Value, key: &str) -> Result<i64, ApiError> {
    required_arg(args, key)?
        .as_i64()
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数")))
}

fn nullable_i64_arg(args: &Value, key: &str) -> Result<Option<i64>, ApiError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数或 null"))),
    }
}

fn u64_arg(args: &Value, key: &str) -> Result<u64, ApiError> {
    required_arg(args, key)?
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数")))
}

fn non_negative_u64_arg(args: &Value, key: &str) -> Result<u64, ApiError> {
    required_arg(args, key)?
        .as_u64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是非负整数")))
}

fn f32_arg(args: &Value, key: &str) -> Result<f32, ApiError> {
    let value = required_arg(args, key)?
        .as_f64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是数字")))?;
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err(ApiError::bad_request(format!("{key} 必须是有限 f32 数字")));
    }
    Ok(value as f32)
}

fn string_arg(args: &Value, key: &str) -> Result<String, ApiError> {
    let value = required_arg(args, key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{key} 不能为空")))?;
    Ok(value.to_string())
}

fn optional_string_field(input: &Value, key: &str) -> Result<Option<String>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ApiError::bad_request(format!("{key} 必须是字符串或 null"))),
    }
}

fn downstream_strategy_arg(args: &Value) -> Result<DownstreamConflictStrategy, ApiError> {
    DownstreamConflictStrategy::from_optional_str(
        optional_string_field(args, "downstreamStrategy")?.as_deref(),
    )
    .map_err(ApiError::bad_request)
}

fn optional_bool_field(input: &Value, key: &str) -> Result<bool, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(ApiError::bad_request(format!("{key} 必须是布尔值或 null"))),
    }
}

fn nullable_string_update_field(
    input: &Value,
    key: &str,
) -> Result<Option<Option<String>>, ApiError> {
    match input.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) => Ok(Some(Some(value.clone()))),
        Some(_) => Err(ApiError::bad_request(format!("{key} 必须是字符串或 null"))),
    }
}

fn nullable_value_update_field(
    input: &Value,
    key: &str,
) -> Result<Option<Option<Value>>, ApiError> {
    match input.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(value) => Ok(Some(Some(value.clone()))),
    }
}

fn optional_i32_field(input: &Value, key: &str) -> Result<Option<i32>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let raw = value
                .as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是整数或 null")))?;
            i32::try_from(raw)
                .map(Some)
                .map_err(|_| ApiError::bad_request(format!("{key} 超出 i32 范围")))
        }
    }
}

fn optional_string_vec_field(input: &Value, key: &str) -> Result<Option<Vec<String>>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是字符串数组或 null")))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(ApiError::bad_request(format!(
            "{key} 必须是字符串数组或 null"
        ))),
    }
}

fn to_value<T: serde::Serialize>(value: T) -> Result<Value, ApiError> {
    serde_json::to_value(value)
        .map_err(|e| ApiError::bad_request(format!("序列化 Web command 响应失败: {e}")))
}

fn is_canvas_web_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "get_canvas_graph"
            | "get_canvas_node_contracts"
            | "save_canvas_graph"
            | "save_canvas_text_asset"
            | "preview_canvas_node_request"
            | "run_canvas_graph"
            | "run_canvas_node"
            | "import_canvas_media"
    )
}

async fn run_canvas_web_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Value, ApiError> {
    match cmd {
        "get_canvas_graph" => get_canvas_graph(state, user_id, args).await,
        "get_canvas_node_contracts" => get_canvas_node_contracts(state, user_id, args).await,
        "save_canvas_graph" => save_canvas_graph(state, user_id, args).await,
        "save_canvas_text_asset" => save_canvas_text_asset(state, user_id, args).await,
        "preview_canvas_node_request" => preview_canvas_node_request(state, user_id, args).await,
        "run_canvas_graph" => run_canvas_graph(state, user_id, args).await,
        "run_canvas_node" => run_canvas_node(state, user_id, args).await,
        "import_canvas_media" => import_canvas_media(state, user_id, args).await,
        _ => unreachable!("is_canvas_web_command filters supported commands"),
    }
}

async fn get_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::get_canvas_graph(
            state,
            user_id,
            command_input(args, "get_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn get_canvas_node_contracts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::get_canvas_node_contracts(
            state,
            user_id,
            command_input(args, "get_canvas_node_contracts")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn save_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::save_canvas_graph(
            state,
            user_id,
            command_input(args, "save_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn save_canvas_text_asset(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::save_canvas_text_asset(
            state,
            user_id,
            command_input(args, "save_canvas_text_asset")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn preview_canvas_node_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::preview_canvas_node_request(
            state,
            user_id,
            command_input(args, "preview_canvas_node_request")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn run_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::run_canvas_graph(
            state,
            user_id,
            command_input(args, "run_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn run_canvas_node(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::run_canvas_node(
            state,
            user_id,
            command_input(args, "run_canvas_node")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn import_canvas_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::import_canvas_media(
            state,
            user_id,
            command_input(args, "import_canvas_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn get_galgame_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::get_galgame_script(
            state,
            user_id,
            command_input(args, "get_galgame_script")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn start_galgame_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let mut input: galgame_records_actions::StartGalgamePipelineInput =
        command_input(args, "start_galgame_pipeline")?;
    input.task_id = sanitize_optional_task_id(input.task_id)?;
    let task_id = input.task_id.clone();
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::GalgameGeneration,
        Some(input.project_id),
        a4n_services::galgame::GALGAME_PIPELINE_TOTAL_STEPS as u32,
        "正在准备 Galgame 管线",
    )
    .await;
    let progress_reporter = web_galgame_progress_reporter(state, task, task_id.clone());
    let result = string_action_result(
        galgame_records_actions::start_galgame_pipeline_with_progress(
            state,
            user_id,
            input,
            Some(Arc::new(progress_reporter)),
        )
        .await,
    );
    to_value(finish_web_task(state, task_id, result, "Galgame 管线已完成").await?)
}

async fn cancel_galgame_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::cancel_galgame_pipeline(
            state,
            user_id,
            command_input(args, "cancel_galgame_pipeline")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn preview_galgame_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::preview_galgame_pipeline_request(
            state,
            user_id,
            command_input(args, "preview_galgame_pipeline_request")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn get_galgame_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::get_galgame_pipeline_status(
            state,
            user_id,
            command_input(args, "get_galgame_pipeline_status")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn update_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_line(
            state,
            user_id,
            project_id,
            command_input(args, "update_galgame_line")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn insert_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::insert_galgame_line(
            state,
            user_id,
            project_id,
            command_input(args, "insert_galgame_line")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn update_galgame_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_scene(
            state,
            user_id,
            project_id,
            command_input(args, "update_galgame_scene")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn delete_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    galgame_records_actions::delete_galgame_line(
        state,
        user_id,
        project_id,
        command_input(args, "delete_galgame_line")?,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

async fn reorder_galgame_lines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::reorder_galgame_lines(
            state,
            user_id,
            project_id,
            command_input(args, "reorder_galgame_lines")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn update_galgame_choices(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_choices(
            state,
            user_id,
            project_id,
            command_input(args, "update_galgame_choices")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn plan_galgame_assets(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::plan_galgame_assets(
            state,
            user_id,
            command_input(args, "plan_galgame_assets")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn run_galgame_asset_generation(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::run_galgame_asset_generation(
            state,
            user_id,
            command_input(args, "run_galgame_asset_generation")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn bind_galgame_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::bind_galgame_media(
            state,
            user_id,
            command_input(args, "bind_galgame_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn unbind_galgame_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::unbind_galgame_media(
            state,
            user_id,
            command_input(args, "unbind_galgame_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn import_galgame_audio(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::import_galgame_audio(
            state,
            user_id,
            command_input(args, "import_galgame_audio")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn export_galgame_webgal(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::export_galgame_webgal(
            state,
            user_id,
            command_input(args, "export_galgame_webgal")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

fn command_input<T>(args: &Value, command: &str) -> Result<T, ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(required_arg(args, "input")?.clone())
        .map_err(|e| ApiError::bad_request(format!("{command}.input 无效: {e}")))
}

fn sanitize_optional_task_id(task_id: Option<String>) -> Result<Option<String>, ApiError> {
    task_id
        .map(|task_id| safe_workflow_task_id(Some(task_id), String::new))
        .transpose()
}

async fn register_web_task(
    state: &ServerState,
    task_id: Option<String>,
    task_type: TaskType,
    project_id: impl Into<Option<i64>>,
    total: u32,
    message: &str,
) -> Option<Arc<TaskHandle>> {
    let task_id = task_id?;
    let handle = state.task_manager.register(TaskHandle {
        task_id: task_id.clone(),
        cancel_token: tokio_util::sync::CancellationToken::new(),
        progress: Arc::new(parking_lot::RwLock::new(TaskProgress::with_meta(
            0,
            total,
            message,
            Some(task_type.progress_meta(message)),
        ))),
        progress_events: Arc::new(parking_lot::RwLock::new(VecDeque::new())),
        task_type,
        project_id: project_id.into(),
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: None,
    });
    emit_workflow_progress(state, &task_id, 0, total, message).await;
    Some(handle)
}

fn spawn_embedding_download_progress_bridge(
    state: &ServerState,
    task_id: &str,
    task: Option<Arc<TaskHandle>>,
) -> tokio::sync::mpsc::UnboundedSender<a4n_engine::embedding::provider::DownloadProgress> {
    let (tx, mut rx) =
        tokio::sync::mpsc::unbounded_channel::<a4n_engine::embedding::provider::DownloadProgress>();
    let state = state.clone();
    let task_id = task_id.to_string();
    let scope = embedding_model_actions::embedding_model_download_task_scope();
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if let Some(handle) = task.as_ref() {
                let total = progress.total_bytes.max(1);
                let current = ((progress.downloaded_bytes.min(total) * u64::from(scope.total))
                    / total)
                    .min(u64::from(scope.total));
                handle.update_progress(current as u32, scope.total, &progress.message);
            }
            emit_workflow_progress_bytes(
                &state,
                &task_id,
                progress.downloaded_bytes,
                progress.total_bytes,
                &progress.message,
            )
            .await;
        }
    });
    tx
}

fn web_progress_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync + 'static {
    let state = state.clone();
    move |current, total, message| {
        let meta = task
            .as_ref()
            .map(|handle| handle.task_type.progress_meta(message));
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(a4n_core::error::AppError::Cancelled);
            }
            handle.update_progress_with_meta(current, total, message, meta.clone());
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let message = message.to_string();
            tokio::spawn(async move {
                emit_workflow_progress_with_meta(&state, &task_id, current, total, &message, meta)
                    .await;
            });
        }
        Ok(())
    }
}

fn web_galgame_progress_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl Fn(
    a4n_services::galgame::GalgamePipelineProgressUpdate,
) -> Result<(), a4n_core::error::AppError>
+ Send
+ Sync
+ 'static {
    let state = state.clone();
    move |event| {
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(a4n_core::error::AppError::Cancelled);
            }
            handle.update_progress_with_meta(
                event.current,
                event.total,
                &event.message,
                Some(event.meta.clone()),
            );
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            tokio::spawn(async move {
                emit_workflow_progress_with_meta(
                    &state,
                    &task_id,
                    event.current,
                    event.total,
                    &event.message,
                    Some(event.meta),
                )
                .await;
            });
        }
        Ok(())
    }
}

fn web_chunk_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl FnMut(&str, u32) -> Result<(), a4n_core::error::AppError> + Send + 'static {
    let state = state.clone();
    move |delta, index| {
        if task.as_ref().is_some_and(|handle| handle.is_cancelled()) {
            return Err(a4n_core::error::AppError::Cancelled);
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let content = delta.to_string();
            tokio::spawn(async move {
                emit_workflow_chunk(&state, &task_id, &content, index).await;
            });
        }
        Ok(())
    }
}

fn web_fire_and_forget_progress_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl Fn(u32, u32, &str) + Send + Sync + 'static {
    let state = state.clone();
    move |current, total, message| {
        let meta = task
            .as_ref()
            .map(|handle| handle.task_type.progress_meta(message));
        if let Some(handle) = task.as_ref() {
            handle.update_progress_with_meta(current, total, message, meta.clone());
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let message = message.to_string();
            tokio::spawn(async move {
                emit_workflow_progress_with_meta(&state, &task_id, current, total, &message, meta)
                    .await;
            });
        }
    }
}

fn string_action_result<T>(result: Result<T, String>) -> Result<T, a4n_core::error::AppError> {
    result.map_err(|message| {
        if message.contains("已停止") || message.contains("已取消") {
            a4n_core::error::AppError::Cancelled
        } else {
            a4n_core::error::AppError::InvalidArgument(message)
        }
    })
}

async fn finish_web_task<T>(
    state: &ServerState,
    task_id: Option<String>,
    result: Result<T, a4n_core::error::AppError>,
    success_message: &str,
) -> Result<T, ApiError> {
    match result {
        Ok(value) => {
            if let Some(task_id) = task_id.as_deref() {
                emit_workflow_complete(state, task_id, Some(success_message.to_string())).await;
                state.task_manager.remove(task_id);
            }
            Ok(value)
        }
        Err(err) => {
            if let Some(task_id) = task_id.as_deref() {
                let is_cancelled = matches!(err, a4n_core::error::AppError::Cancelled);
                emit_workflow_error(
                    state,
                    task_id,
                    if is_cancelled {
                        "cancelled"
                    } else {
                        "workflow_failed"
                    },
                    &err.to_string(),
                    false,
                )
                .await;
                state.task_manager.remove(task_id);
            }
            Err(err.into())
        }
    }
}

async fn finish_web_video_generation_task<F>(
    state: &ServerState,
    task_id: Option<String>,
    result: Result<video_generation_actions::GeneratedVideoSummary, String>,
    error_code: &str,
    success_summary: F,
) -> Result<video_generation_actions::GeneratedVideoSummary, ApiError>
where
    F: FnOnce(&video_generation_actions::GeneratedVideoSummary) -> String,
{
    match result {
        Ok(video) => {
            if let Some(task_id) = task_id.as_deref() {
                if video.status == "failed" {
                    emit_workflow_error(
                        state,
                        task_id,
                        error_code,
                        &success_summary(&video),
                        false,
                    )
                    .await;
                } else {
                    emit_workflow_complete(state, task_id, Some(success_summary(&video))).await;
                }
                state.task_manager.remove(task_id);
            }
            Ok(video)
        }
        Err(err) => {
            if let Some(task_id) = task_id.as_deref() {
                let is_cancelled = err.contains("已停止") || err.contains("已取消");
                emit_workflow_error(
                    state,
                    task_id,
                    if is_cancelled {
                        "cancelled"
                    } else {
                        error_code
                    },
                    &err,
                    false,
                )
                .await;
                state.task_manager.remove(task_id);
            }
            Err(ApiError::bad_request(err))
        }
    }
}

fn required_json_i32(value: &Value, key: &str, label: &str) -> Result<i32, ApiError> {
    let raw = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是整数")))?;
    i32::try_from(raw).map_err(|_| ApiError::bad_request(format!("{label}.{key} 超出 i32 范围")))
}
