mod admin;
mod canvas;
mod characters;
mod fact_feedback;
mod front;
mod galgame;
mod import_project;
mod input;
mod media;
mod notifications;
mod package;
mod request;
mod source_sync;
mod task;

mod dispatch {
    mod assets_and_media;
    mod config_and_rag;
    mod galgame;
    mod manga;
    mod media_authoring;
    mod projects_and_content;
    mod system_and_collaboration;
    mod video;
    mod visual_plan;

    use super::*;
    use a4n_runtime::host_api::agent_runtime::respond_agent_editor;

    pub(super) async fn dispatch(
        state: &ServerState,
        user_id: i64,
        args: &Value,
        cmd: &str,
    ) -> Result<Value, ApiError> {
        // Editor replies can execute an approved tool; do not nest that work
        // under the large system-command match's debug poll stack.
        if cmd == "respond_agent_editor" {
            let input = strict_command_input(args, cmd)?;
            let reply = respond_agent_editor(state.runtime(), user_id, input).await?;
            return Ok(serde_json::json!(reply));
        }
        if cmd == "review_agent_delegation" {
            let input = strict_command_input(args, cmd)?;
            let reply = a4n_runtime::host_api::agent_runtime::review_delegation(
                state.runtime(),
                user_id,
                input,
            )
            .await?;
            return Ok(serde_json::json!(reply));
        }
        if let Some(result) = Box::pin(super::notifications::try_dispatch(
            state, user_id, args, cmd,
        ))
        .await?
        {
            return Ok(result);
        }
        if let Some(result) =
            Box::pin(config_and_rag::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(visual_plan::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(projects_and_content::try_dispatch(
            state, user_id, args, cmd,
        ))
        .await?
        {
            return Ok(result);
        }
        if let Some(result) =
            Box::pin(assets_and_media::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(video::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) = Box::pin(manga::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) =
            Box::pin(media_authoring::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(galgame::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) = Box::pin(system_and_collaboration::try_dispatch(
            state, user_id, args, cmd,
        ))
        .await?
        {
            return Ok(result);
        }
        Err(ApiError::bad_request(format!(
            "Web command 尚未实现: {cmd}"
        )))
    }
}

use self::canvas::{is_canvas_web_command, run_canvas_web_command};
use self::characters::{
    add_profile_attribute, apply_implicit_update_decision, apply_optimization_patches,
    attribute_history, check_character_conflict, check_implicit_update,
    check_profile_sync_conflict, create_optimization_analysis_session,
    create_optimization_chat_session, create_profile, delete_optimize_session, delete_profile,
    diff_profile_chapters, discard_optimization_patch_set, discard_pending_optimize_patch_sets,
    discover_character_profiles, execute_attribute_deletion, get_character_arc_status,
    get_character_voice_fingerprint, get_implicit_review, get_inspiration_history,
    get_optimize_session, get_profile_state_at_chapter, get_snapshot, list_behaviors,
    list_deletion_marks, list_optimize_patch_sets, list_optimize_sessions, list_optimize_turns,
    list_profiles, list_snapshots, modify_profile_attribute, prepare_optimization_analysis_session,
    preview_check_character_conflict_request, preview_check_implicit_update_request,
    preview_create_optimization_analysis_session_request,
    preview_discover_character_profiles_request, preview_inspiration_request,
    preview_prepare_optimization_analysis_session_request,
    preview_propose_optimization_patches_request, preview_send_optimization_message_request,
    preview_sync_all_character_profiles_request, preview_sync_character_from_chapters_request,
    record_optimize_agent_event, request_attribute_deletion, reset_deletion_marks,
    rollback_to_chapter, sync_all_character_profiles, sync_character_from_chapters,
    update_character_profile,
};
use self::front::{
    activate_theme, add_image_config, add_video_config, cancel_embedding_model_download,
    cancel_generation_task, cancel_task, cancel_workflow, change_user_password,
    create_embedding_config, create_llm_config, create_theme, delete_embedding_config,
    delete_llm_config, delete_local_embedding_model, delete_theme, delete_theme_resource,
    detect_image_config_models, detect_llm_config_models, detect_video_config_models,
    download_embedding_model, export_all_configs_json, export_theme, export_theme_package,
    get_current_user, get_embedding_model_download_info, get_queue_stats, get_task_history,
    get_theme_preference, get_workflow_progress_events, get_workflow_run_detail,
    import_all_configs, import_theme, import_theme_package, list_active_workflows,
    list_agent_skills, list_embedding_config, list_image_configs, list_llm_config,
    list_local_embedding_models, list_model_provider_catalog, list_sessions, list_themes,
    list_video_configs, list_workflow_runs, login_user, oauth2_authorize_url,
    oauth2_available_providers, oauth2_callback, preview_test_embedding_config_request,
    preview_test_image_config_request, preview_test_llm_config_request, prompt_assembly_get,
    prompt_assembly_list, prompt_assembly_reset, prompt_assembly_save, prompt_list,
    prompt_list_categories, prompt_list_project_overrides, prompt_load_defaults,
    prompt_remove_project_override, prompt_render, prompt_render_ab, prompt_reset,
    prompt_set_project_override, prompt_update, prompt_validate_variables, prompt_version_diff,
    prompt_version_history, rag_enhanced_search, rag_enhanced_search_with_vectors, rag_ingest_all,
    rag_prepare_enhanced_search, rag_prepare_ingest_all, rag_upload_ingest_embeddings,
    refresh_token, register_user, remove_image_config, remove_video_config, resume_workflow_run,
    revoke_session, set_default_embedding_config, set_default_image_config, set_default_llm_config,
    set_default_video_config, test_embedding_config, test_image_config, test_llm_config,
    update_agent_skill_configuration, update_embedding_config, update_image_config,
    update_llm_config, update_theme, update_theme_preference, update_user_profile,
    update_video_config, upload_theme_resource, web_fetch, web_search,
};
use self::galgame::{
    advance_galgame_story_runtime, bind_galgame_media, cancel_galgame_pipeline,
    delete_galgame_line, export_galgame_webgal, get_galgame_asset_plan,
    get_galgame_pipeline_status, get_galgame_script, get_galgame_story_runtime_state,
    import_galgame_audio, insert_galgame_line, plan_galgame_assets,
    preview_galgame_pipeline_request, reorder_galgame_lines, run_galgame_asset_generation,
    save_galgame_story_graph, start_galgame_pipeline, start_galgame_story_runtime,
    unbind_galgame_media, update_galgame_choices, update_galgame_line, update_galgame_scene,
};
use self::import_project::{
    accept_import_review, accept_project_invite, batch_accept_reviews, cancel_import_analysis,
    check_import_dedup, cleanup_orphan_project_data_dirs, confirm_chapter_visual_plan,
    create_canvas_project, create_derived_project, create_project, create_standalone_comic_project,
    decline_project_invite, delete_chapter_scene, delete_chapter_shooting_script,
    delete_import_review, delete_latest_chapter_outlines, delete_latest_parts, delete_project,
    diff_blueprint_versions, discard_chapter_visual_plan_draft, duplicate_project,
    edit_chapter_shooting_script, edit_chapter_shooting_script_progress, export_batch_data,
    export_canvas_project_data, export_characters_json_data, export_derived_project_data,
    export_epub_data, export_markdown_data, export_outline_json_data, export_pdf_data,
    export_project_bundle_data, export_project_family_data, export_txt_data,
    forward_project_status, generate_chapter_analysis, generate_chapter_scene,
    generate_portrait_prompt, get_blueprint, get_chapter, get_chapter_outlines_pacing,
    get_chapter_shooting_script, get_chapter_shooting_script_unit_progress,
    get_chapter_visual_plan, get_generation_call_analytics, get_generation_progress,
    get_import_review, get_log_dir, get_log_level, get_model_usage_report, get_primary_portrait,
    get_project,
    get_project_asset_dependency_graph, get_project_asset_dependency_graph_node_detail,
    get_project_import_analysis_progress, get_project_status_machine, get_source_snapshot_chapter,
    get_storage_paths, import_canvas_project_upload, import_chapter, import_project_bundle_upload,
    import_project_family_upload, import_project_text_asset_upload, import_project_text_upload,
    inspect_project_bundle_upload, invite_project_member, is_reverse_engineer_web_command,
    leave_project, list_blueprint_versions, list_chapter_analyses, list_chapter_index,
    list_chapter_outline_index, list_chapter_outlines, list_chapter_scenes, list_chapters,
    list_import_queue, list_import_queue_index, list_my_project_invites,
    list_orphan_project_data_dirs, list_parts, list_portraits_by_project, list_project_invites,
    list_project_members, list_projects, list_source_snapshot_chapter_index,
    list_source_snapshot_chapters, list_story_event_chapters,
    prepare_chapter_generation_rag_embedding, preview_apply_beat_mapping_request,
    preview_batch_generate_part_outlines_request, preview_chapter_analysis_request,
    preview_chapter_prompt, preview_chapter_scene_request, preview_continue_part_outlines_request,
    preview_generate_age_variant_request, preview_generate_blueprint_request,
    preview_generate_chapter_outlines_by_count_request, preview_generate_missing_portraits_request,
    preview_generate_part_chapter_outlines_request, preview_generate_parts_request,
    preview_generate_portrait_prompt_request, preview_generate_portrait_request,
    preview_import_project_analysis_request, preview_refine_blueprint_request,
    preview_regenerate_chapter_outline_request, preview_regenerate_last_part_request,
    preview_regenerate_part_outline_request, preview_regenerate_parts_request,
    preview_regenerate_portrait_request, preview_resume_chapter_analysis_sync_request,
    preview_rollback, read_recent_logs, reject_import_review, remove_project_member,
    reorder_chapter_scenes, reset_project_generated_data, restore_blueprint_version,
    resume_chapter_analysis_sync, revoke_project_invite, rollback_project_status,
    run_import_project_analysis, run_reverse_engineer_web_command, save_blueprint,
    save_chapter_outline, save_chapter_visual_plan_draft, save_part_outline, set_log_level,
    set_primary_portrait, sync_blueprint_materials, sync_outline_materials, update_chapter_content,
    update_chapter_outline, update_chapter_scene, update_project, update_project_member_role,
    update_world_setting, upload_portrait,
};
use self::import_project::{
    edit_chapter_visual_plan_draft, preview_visual_plan_point_generation, read_chapter_visual_plan,
    read_visual_plan_prompt_candidate, validate_chapter_visual_plan,
};
use self::input::{
    bool_arg, command_input, direct_command_input, empty_command_args, f32_arg, i64_arg,
    nullable_i64_arg, optional_i32_field, optional_string_field, reject_unknown_command_args,
    required_arg, required_json_i32, strict_command_input,
    strict_optional_task_runtime_command_input, strict_runtime_command_input, string_arg, to_value,
    u64_arg,
};
use self::media::{
    apply_assembly_postprocess_edit, assemble_chapter_videos, bind_manga_reference_image,
    cancel_manga_pipeline, commit_inserted_segment_draft, create_foreshadowing,
    create_object_profile, create_scene_profile, delete_chapter_video_prompt, delete_foreshadowing,
    delete_image, delete_inserted_segment, delete_manga_by_chapter, delete_manga_page,
    delete_manga_pipeline_artifact, delete_manga_prompt, delete_object_profile,
    delete_scene_profile, delete_video, generate_object_profile_image_prompt,
    generate_scene_profile_image_prompt, get_chapter_video_timeline, get_manga_pipeline_status,
    list_chapter_video_prompts, list_foreshadowing, list_images_by_chapter, list_images_by_project,
    list_images_by_projects, list_manga_page_prompts, list_manga_pipeline_artifacts,
    list_manga_pipeline_statuses, list_object_profiles, list_profile_materials,
    list_scene_profiles, list_video_reference_images, list_videos_by_chapter,
    load_cached_assembled_chapter_video_preview, load_video_playback_data,
    load_video_scrub_playback_data, load_video_segment_plan_draft, move_inserted_segment,
    optimize_chapter_video_prompt, optimize_manga_prompt, poll_video_job,
    prepare_standalone_comic_source, preview_batch_images_request, preview_chapter_video_request,
    preview_delete_manga_pipeline_artifact, preview_object_profile_image_request,
    preview_page_image_request, preview_reference_media_video_request,
    preview_resume_manga_pipeline_request, preview_scene_image_request,
    preview_start_manga_pipeline_request, preview_video_provider_request_by_video_id,
    push_assembled_chapter_video, read_image, resume_manga_pipeline, save_manga_pipeline_artifact,
    select_segment_video, set_profile_primary_material, start_manga_pipeline, sync_scene_profiles,
    trim_video, update_assembly_push_status, update_chapter_video_prompt,
    update_chapter_video_segment_plan, update_chapter_video_segment_recipe, update_foreshadowing,
    update_image, update_manga_prompt, update_object_profile, update_scene_profile, update_video,
    update_video_segment_plan_draft_recipe, upload_assembly_insert_video, upload_chapter_video,
    upload_image, upload_profile_material_image, upload_reference_media_video,
};
use self::request::run_with_runtime_context;
use self::source_sync::{apply_source_sync_decisions, detach_derived_project, preview_source_sync};
use self::task::{
    finish_web_task, register_web_task, sanitize_optional_task_id, sanitize_task_id,
    string_action_result, web_fire_and_forget_progress_reporter, web_progress_reporter,
    web_stream_delta_reporter, web_stream_reporter,
};

use super::auth::{authenticated_user, user_agent};
use super::error::ApiError;
use crate::state::ServerState;
use a4n_domain::task_queue::TaskKind;
use a4n_runtime::host_api::auth_records as auth_records_actions;
use a4n_runtime::host_api::blueprint_records as blueprint_records_actions;
use a4n_runtime::host_api::chapter_content as chapter_content_actions;
use a4n_runtime::host_api::chapter_generation as chapter_generation_actions;
use a4n_runtime::host_api::chapter_scene as chapter_scene_actions;
use a4n_runtime::host_api::character_ai as character_ai_actions;
use a4n_runtime::host_api::character_conflict as character_conflict_actions;
use a4n_runtime::host_api::character_implicit_review as character_implicit_review_actions;
use a4n_runtime::host_api::character_records as character_records_actions;
use a4n_runtime::host_api::config_records as config_records_actions;
use a4n_runtime::host_api::embedding_model as embedding_model_actions;
use a4n_runtime::host_api::export_files as export_files_actions;
use a4n_runtime::host_api::foreshadowing_records as foreshadowing_records_actions;
use a4n_runtime::host_api::generation_analytics as generation_analytics_actions;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use a4n_runtime::host_api::image_records as image_records_actions;
use a4n_runtime::host_api::import_analysis as import_analysis_actions;
use a4n_runtime::host_api::import_file as import_file_actions;
use a4n_runtime::host_api::import_reverse as import_reverse_actions;
use a4n_runtime::host_api::import_review as import_review_actions;
use a4n_runtime::host_api::inspiration_records as inspiration_records_actions;
use a4n_runtime::host_api::logging as logging_actions;
use a4n_runtime::host_api::manga_pipeline as manga_pipeline_actions;
use a4n_runtime::host_api::manga_records as manga_records_actions;
use a4n_runtime::host_api::oauth as oauth_actions;
use a4n_runtime::host_api::object_profile_records as object_profile_records_actions;
use a4n_runtime::host_api::optimize as optimize_actions;
use a4n_runtime::host_api::outline_records as outline_records_actions;
use a4n_runtime::host_api::package as package_actions;
use a4n_runtime::host_api::portrait_generation as portrait_generation_actions;
use a4n_runtime::host_api::portrait_records as portrait_records_actions;
use a4n_runtime::host_api::profile_materials as profile_materials_actions;
use a4n_runtime::host_api::project_collaboration as project_collaboration_actions;
use a4n_runtime::host_api::project_records as project_records_actions;
use a4n_runtime::host_api::project_status as project_status_actions;
use a4n_runtime::host_api::project_transfer as project_transfer_actions;
use a4n_runtime::host_api::prompt_records as prompt_records_actions;
use a4n_runtime::host_api::rag_search as rag_search_actions;
use a4n_runtime::host_api::scene_profile_records as scene_profile_records_actions;
use a4n_runtime::host_api::scene_profile_sync as scene_profile_sync_actions;
use a4n_runtime::host_api::source_sync as source_sync_actions;
use a4n_runtime::host_api::storage_paths as storage_paths_actions;
use a4n_runtime::host_api::task_records as task_records_actions;
use a4n_runtime::host_api::theme as theme_actions;
use a4n_runtime::host_api::video_generation as video_generation_actions;
use a4n_runtime::host_api::video_prompts as video_prompt_actions;
use a4n_runtime::host_api::video_records as video_records_actions;
use a4n_worker_contract::{AgentHostCommandPayload, WorkerEnvelope};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

pub async fn invoke(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(cmd): Path<String>,
    Json(args): Json<Value>,
) -> Result<Response, ApiError> {
    let public_result = match cmd.as_str() {
        "login_user" => Some(login_user(&state, &headers, &args).await?),
        "register_user" => Some(register_user(&state, &headers, &args).await?),
        "refresh_token" => Some(refresh_token(&state, &args).await?),
        "oauth2_authorize_url" => Some(oauth2_authorize_url(&state, &args).await?),
        "oauth2_available_providers" => Some(oauth2_available_providers(&state, &args).await?),
        "oauth2_callback" => Some(oauth2_callback(&state, &headers, &args).await?),
        _ => None,
    };
    if let Some(result) = public_result {
        return Ok(Json(result).into_response());
    }

    let user = authenticated_user(&state, &headers).await?;
    let result = dispatch_runtime_command_inner(state.runtime(), user.id, args, cmd, None).await?;

    Ok(Json(result).into_response())
}

pub async fn dispatch_runtime_command_value(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    args: Value,
    cmd: String,
) -> Result<Value, Value> {
    dispatch_runtime_command_inner(runtime, user_id, args, cmd, None)
        .await
        .map_err(ApiError::into_value)
}

pub async fn dispatch_agent_host_command(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    request: WorkerEnvelope<AgentHostCommandPayload>,
) -> Result<Value, String> {
    dispatch_agent_host_command_inner(runtime, user_id, request)
        .await
        .map_err(|error| error.to_string())
}

pub async fn dispatch_agent_host_command_value(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    request: WorkerEnvelope<AgentHostCommandPayload>,
) -> Result<Value, ApiError> {
    dispatch_agent_host_command_inner(runtime, user_id, request).await
}

async fn dispatch_agent_host_command_inner(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    request: WorkerEnvelope<AgentHostCommandPayload>,
) -> Result<Value, ApiError> {
    request
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    request
        .payload
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let command = request.payload.command.clone();
    let mut arguments = request
        .payload
        .arguments_with_runtime_snapshot()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    if matches!(
        command.as_str(),
        "save_chapter_visual_plan_draft" | "edit_chapter_visual_plan_draft"
    ) {
        let input = arguments
            .get_mut("input")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| ApiError::bad_request("导演编辑必须提供 input"))?;
        if input.get("operation_id").is_none_or(Value::is_null) {
            let execution_id = request
                .payload
                .execution_id
                .ok_or_else(|| ApiError::bad_request("导演编辑缺少持久 execution_id"))?;
            input.insert(
                "operation_id".into(),
                Value::String(format!("agent:{execution_id}")),
            );
        }
    }
    // The manifest is the exposure gate for both kinds of Agent tools. Only
    // synthetic tools belong to the Agent-native dispatcher; ordinary
    // manifest tools must retain their existing command dispatch path.
    a4n_api_contract::agent::agent_execution_tool_descriptor(&command).ok_or_else(|| {
        ApiError::bad_request(format!(
            "Host command manifest 禁止 Agent 使用工具或工具不存在: {command}"
        ))
    })?;
    let agent_context =
        a4n_api_contract::agent::synthetic_agent_tool_descriptor(&command).map(|_| {
            AgentHostDispatchContext {
                scope: request.payload.scope.clone(),
                run_id: request.payload.run_id,
                turn_id: request.payload.turn_id,
                execution_id: request.payload.execution_id,
                lease_token: request.lease.fence_token.clone(),
            }
        });
    dispatch_runtime_command_inner(runtime, user_id, arguments, command, agent_context).await
}

struct AgentHostDispatchContext {
    scope: a4n_api_contract::agent::AgentScope,
    run_id: i64,
    turn_id: i64,
    execution_id: Option<i64>,
    lease_token: String,
}

async fn dispatch_runtime_command_inner(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    args: Value,
    cmd: String,
    agent_context: Option<AgentHostDispatchContext>,
) -> Result<Value, ApiError> {
    let runtime_config_snapshot = a4n_runtime::host_api::runtime_config::snapshot_from_args(&args)
        .map_err(ApiError::bad_request)?;
    let mut command_args = args;
    if let Value::Object(fields) = &mut command_args {
        fields.remove("runtime_config_snapshot");
    }
    let state = ServerState::from_runtime(runtime.clone());
    if agent_context.is_none() {
        let availability = package_actions::command_availability(runtime, user_id, &cmd)
            .await
            .map_err(ApiError::from)?;
        if !availability.available {
            return Err(ApiError::package_unavailable(availability));
        }
    }

    run_with_runtime_context(user_id, runtime_config_snapshot, async move {
        if let Some(context) = agent_context {
            return a4n_runtime::host_api::agent_runtime::dispatch_agent_host_command(
                state.runtime(),
                user_id,
                context.scope,
                context.run_id,
                context.turn_id,
                context.execution_id,
                context.lease_token,
                cmd,
                command_args,
            )
            .await
            .map_err(ApiError::from);
        }
        if is_reverse_engineer_web_command(&cmd) {
            return run_reverse_engineer_web_command(&state, user_id, &command_args, &cmd).await;
        }
        if is_canvas_web_command(&cmd) {
            return run_canvas_web_command(&state, user_id, &command_args, &cmd).await;
        }
        dispatch::dispatch(&state, user_id, &command_args, &cmd).await
    })
    .await
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::agent;

    #[test]
    fn agent_host_route_keeps_manifest_and_synthetic_tools_separate() {
        assert!(agent::agent_tool_descriptor("read_chapter_visual_plan").is_some());
        assert!(
            agent::synthetic_agent_tool_descriptor("read_chapter_visual_plan").is_none(),
            "ordinary manifest tools must use the regular command dispatcher"
        );
        assert!(agent::synthetic_agent_tool_descriptor("read_agent_memory").is_some());
        assert!(agent::agent_tool_descriptor("create_llm_config").is_none());
        assert!(agent::agent_tool_descriptor("missing_agent_tool").is_none());
    }
}
