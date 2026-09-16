use super::super::{
    ApiError, ServerState, accept_import_review, add_profile_attribute, admin,
    apply_assembly_postprocess_edit, apply_implicit_update_decision, apply_optimization_patches,
    attribute_history, batch_accept_reviews, cancel_generation_task, cancel_import_analysis,
    cancel_task, cancel_workflow, change_user_password, check_implicit_update, check_import_dedup,
    check_profile_sync_conflict, create_optimization_analysis_session,
    create_optimization_chat_session, create_profile, delete_import_review, delete_latest_parts,
    delete_profile, execute_attribute_deletion, export_batch_data, export_epub_data,
    export_markdown_data, export_pdf_data, export_txt_data, get_current_user,
    get_generation_call_analytics, get_implicit_review, get_log_dir, get_model_usage_report,
    get_queue_stats, get_snapshot, get_task_history, get_workflow_progress_events,
    get_workflow_run_detail, import_project_text_upload, list_active_workflows, list_behaviors,
    list_deletion_marks, list_import_queue, list_parts, list_profiles, list_sessions,
    list_snapshots, list_workflow_runs, modify_profile_attribute, package,
    prepare_optimization_analysis_session, preview_check_implicit_update_request,
    preview_create_optimization_analysis_session_request, preview_generate_parts_request,
    preview_prepare_optimization_analysis_session_request,
    preview_propose_optimization_patches_request, preview_regenerate_last_part_request,
    preview_regenerate_parts_request, preview_send_optimization_message_request, prompt_list,
    prompt_list_categories, prompt_load_defaults, prompt_render, prompt_render_ab, prompt_reset,
    prompt_update, prompt_validate_variables, prompt_version_diff, prompt_version_history,
    read_recent_logs, reject_import_review, request_attribute_deletion, reset_deletion_marks,
    resume_workflow_run, revoke_session, set_log_level, set_profile_primary_material,
    update_assembly_push_status, update_character_profile, update_user_profile,
    update_world_setting,
};
use serde_json::Value;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "list_all_users" => admin::list_all_users(state, user_id, args).await?,
        "create_user" => admin::create_user(state, user_id, args).await?,
        "list_invite_codes" => admin::list_invite_codes(state, user_id, args).await?,
        "create_invite_code" => admin::create_invite_code(state, user_id, args).await?,
        "update_user_role" => admin::update_user_role(state, user_id, args).await?,
        "disable_user" => admin::disable_user(state, user_id, args).await?,
        "get_current_user" => get_current_user(state, user_id, args).await?,
        "update_user_profile" => update_user_profile(state, user_id, args).await?,
        "change_user_password" => change_user_password(state, user_id, args).await?,
        "list_sessions" => list_sessions(state, user_id, args).await?,
        "revoke_session" => revoke_session(state, user_id, args).await?,
        "prompt_render" => prompt_render(state, user_id, args).await?,
        "prompt_list" => prompt_list(state, user_id, args).await?,
        "prompt_load_defaults" => prompt_load_defaults(state, user_id, args).await?,
        "prompt_update" => prompt_update(state, user_id, args).await?,
        "prompt_reset" => prompt_reset(state, user_id, args).await?,
        "prompt_version_history" => prompt_version_history(state, user_id, args).await?,
        "prompt_version_diff" => prompt_version_diff(state, user_id, args).await?,
        "prompt_validate_variables" => prompt_validate_variables(state, user_id, args).await?,
        "prompt_render_ab" => prompt_render_ab(state, user_id, args).await?,
        "prompt_list_categories" => prompt_list_categories(state, user_id, args).await?,
        "list_package_command_availability" => {
            package::list_package_command_availability(state, user_id, args).await?
        }
        "get_package_overview" => package::get_package_overview(state, user_id, args).await?,
        "get_package_operation" => package::get_package_operation(state, user_id, args).await?,
        "start_package_lifecycle" => package::start_package_lifecycle(state, user_id, args).await?,
        "get_package_preferences" => package::get_package_preferences(state, user_id, args).await?,
        "update_package_preferences" => {
            package::update_package_preferences(state, user_id, args).await?
        }
        "get_user_usage" => admin::get_user_usage(state, user_id, args).await?,
        "get_token_summary" => admin::get_token_summary(state, user_id, args).await?,
        "list_pending_tasks" => admin::list_pending_tasks(state, user_id, args).await?,
        "cancel_admin_task" => admin::cancel_admin_task(state, user_id, args).await?,
        "health_check" => admin::health_check(state, user_id, args).await?,
        "get_usage_trends" => admin::get_usage_trends(state, user_id, args).await?,
        "get_admin_overview" => admin::get_admin_overview(state, user_id, args).await?,
        "list_active_workflows" => list_active_workflows(state, user_id, args).await?,
        "list_workflow_runs" => list_workflow_runs(state, user_id, args).await?,
        "get_workflow_run_detail" => get_workflow_run_detail(state, user_id, args).await?,
        "resume_workflow_run" => resume_workflow_run(state, user_id, args).await?,
        "get_workflow_progress_events" => {
            get_workflow_progress_events(state, user_id, args).await?
        }
        "cancel_workflow" => cancel_workflow(state, user_id, args).await?,
        "cancel_task" => cancel_task(state, user_id, args).await?,
        "cancel_generation_task" => cancel_generation_task(state, user_id, args).await?,
        "get_queue_stats" => get_queue_stats(state, user_id, args).await?,
        "get_task_history" => get_task_history(state, user_id, args).await?,
        "set_profile_primary_material" => {
            set_profile_primary_material(state, user_id, args).await?
        }
        "update_assembly_push_status" => update_assembly_push_status(state, user_id, args).await?,
        "apply_assembly_postprocess_edit" => {
            apply_assembly_postprocess_edit(state, user_id, args).await?
        }
        "list_profiles" => list_profiles(state, user_id, args).await?,
        "create_profile" => create_profile(state, user_id, args).await?,
        "update_character_profile" => update_character_profile(state, user_id, args).await?,
        "delete_profile" => delete_profile(state, user_id, args).await?,
        "add_profile_attribute" => add_profile_attribute(state, user_id, args).await?,
        "modify_profile_attribute" => modify_profile_attribute(state, user_id, args).await?,
        "attribute_history" => attribute_history(state, user_id, args).await?,
        "create_optimization_chat_session" => {
            create_optimization_chat_session(state, user_id, args).await?
        }
        "create_optimization_analysis_session" => {
            create_optimization_analysis_session(state, user_id, args).await?
        }
        "prepare_optimization_analysis_session" => {
            prepare_optimization_analysis_session(state, user_id, args).await?
        }
        "preview_create_optimization_analysis_session_request" => {
            preview_create_optimization_analysis_session_request(state, user_id, args).await?
        }
        "preview_prepare_optimization_analysis_session_request" => {
            preview_prepare_optimization_analysis_session_request(state, user_id, args).await?
        }
        "preview_propose_optimization_patches_request" => {
            preview_propose_optimization_patches_request(state, user_id, args).await?
        }
        "apply_optimization_patches" => apply_optimization_patches(state, user_id, args).await?,
        "preview_send_optimization_message_request" => {
            preview_send_optimization_message_request(state, user_id, args).await?
        }
        "import_project_text_upload" => import_project_text_upload(state, user_id, args).await?,
        "list_import_queue" => list_import_queue(state, user_id, args).await?,
        "delete_import_review" => delete_import_review(state, user_id, args).await?,
        "accept_import_review" => accept_import_review(state, user_id, args).await?,
        "reject_import_review" => reject_import_review(state, user_id, args).await?,
        "batch_accept_reviews" => batch_accept_reviews(state, user_id, args).await?,
        "cancel_import_analysis" => cancel_import_analysis(state, user_id, args).await?,
        "check_import_dedup" => check_import_dedup(state, user_id, args).await?,
        "export_txt_data" => export_txt_data(state, user_id, args).await?,
        "export_markdown_data" => export_markdown_data(state, user_id, args).await?,
        "export_epub_data" => export_epub_data(state, user_id, args).await?,
        "export_pdf_data" => export_pdf_data(state, user_id, args).await?,
        "export_batch_data" => export_batch_data(state, user_id, args).await?,
        "list_snapshots" => list_snapshots(state, user_id, args).await?,
        "get_snapshot" => get_snapshot(state, user_id, args).await?,
        "check_profile_sync_conflict" => check_profile_sync_conflict(state, user_id, args).await?,
        "list_behaviors" => list_behaviors(state, user_id, args).await?,
        "get_implicit_review" => get_implicit_review(state, user_id, args).await?,
        "check_implicit_update" => check_implicit_update(state, user_id, args).await?,
        "preview_check_implicit_update_request" => {
            preview_check_implicit_update_request(state, user_id, args).await?
        }
        "apply_implicit_update_decision" => {
            apply_implicit_update_decision(state, user_id, args).await?
        }
        "list_deletion_marks" => list_deletion_marks(state, user_id, args).await?,
        "request_attribute_deletion" => request_attribute_deletion(state, user_id, args).await?,
        "execute_attribute_deletion" => execute_attribute_deletion(state, user_id, args).await?,
        "reset_deletion_marks" => reset_deletion_marks(state, user_id, args).await?,
        "get_log_dir" => get_log_dir(state, user_id, args).await?,
        "read_recent_logs" => read_recent_logs(state, user_id, args).await?,
        "set_log_level" => set_log_level(state, user_id, args).await?,
        "get_generation_call_analytics" => {
            get_generation_call_analytics(state, user_id, args).await?
        }
        "get_model_usage_report" => get_model_usage_report(state, user_id, args).await?,
        "update_world_setting" => update_world_setting(state, user_id, args).await?,
        "list_parts" => list_parts(state, user_id, args).await?,
        "preview_generate_parts_request" => {
            preview_generate_parts_request(state, user_id, args).await?
        }
        "delete_latest_parts" => delete_latest_parts(state, user_id, args).await?,
        "preview_regenerate_parts_request" => {
            preview_regenerate_parts_request(state, user_id, args).await?
        }
        "preview_regenerate_last_part_request" => {
            preview_regenerate_last_part_request(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
