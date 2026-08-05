use super::super::*;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "prompt_set_project_override" => prompt_set_project_override(state, user_id, args).await?,
        "prompt_remove_project_override" => {
            prompt_remove_project_override(state, user_id, args).await?
        }
        "prompt_list_project_overrides" => {
            prompt_list_project_overrides(state, user_id, args).await?
        }
        "list_all_projects" => admin::list_all_projects(state, user_id, args).await?,
        "list_optimize_sessions" => list_optimize_sessions(state, user_id, args).await?,
        "get_optimize_session" => get_optimize_session(state, user_id, args).await?,
        "list_optimize_turns" => list_optimize_turns(state, user_id, args).await?,
        "list_optimize_patch_sets" => list_optimize_patch_sets(state, user_id, args).await?,
        "reject_pending_optimize_patch_sets" => {
            reject_pending_optimize_patch_sets(state, user_id, args).await?
        }
        "delete_optimize_session" => delete_optimize_session(state, user_id, args).await?,
        "record_optimize_agent_event" => record_optimize_agent_event(state, user_id, args).await?,
        "get_inspiration_history" => get_inspiration_history(state, user_id, args).await?,
        "preview_inspiration_request" => preview_inspiration_request(state, user_id, args).await?,
        "get_project_import_analysis_progress" => {
            get_project_import_analysis_progress(state, user_id, args).await?
        }
        "run_import_project_analysis" => run_import_project_analysis(state, user_id, args).await?,
        "preview_import_project_analysis_request" => {
            preview_import_project_analysis_request(state, user_id, args).await?
        }
        "export_outline_json_data" => export_outline_json_data(state, user_id, args).await?,
        "get_profile_state_at_chapter" => {
            get_profile_state_at_chapter(state, user_id, args).await?
        }
        "diff_profile_chapters" => diff_profile_chapters(state, user_id, args).await?,
        "rollback_to_chapter" => rollback_to_chapter(state, user_id, args).await?,
        "list_orphan_project_data_dirs" => {
            list_orphan_project_data_dirs(state, user_id, args).await?
        }
        "cleanup_orphan_project_data_dirs" => {
            cleanup_orphan_project_data_dirs(state, user_id, args).await?
        }
        "update_project" => update_project(state, user_id, args).await?,
        "delete_project" => delete_project(state, user_id, args).await?,
        "reset_project_generated_data" => {
            reset_project_generated_data(state, user_id, args).await?
        }
        "list_project_members" => list_project_members(state, user_id, args).await?,
        "invite_project_member" => invite_project_member(state, user_id, args).await?,
        "list_project_invites" => list_project_invites(state, user_id, args).await?,
        "list_my_project_invites" => list_my_project_invites(state, user_id, args).await?,
        "accept_project_invite" => accept_project_invite(state, user_id, args).await?,
        "decline_project_invite" => decline_project_invite(state, user_id, args).await?,
        "revoke_project_invite" => revoke_project_invite(state, user_id, args).await?,
        "update_project_member_role" => update_project_member_role(state, user_id, args).await?,
        "remove_project_member" => remove_project_member(state, user_id, args).await?,
        "leave_project" => leave_project(state, user_id, args).await?,
        "import_project_full_upload" => import_project_full_upload(state, user_id, args).await?,
        "export_project_family_data" => export_project_family_data(state, user_id, args).await?,
        "export_derived_project_data" => export_derived_project_data(state, user_id, args).await?,
        "import_project_family_upload" => {
            import_project_family_upload(state, user_id, args).await?
        }
        "forward_project_status" => forward_project_status(state, user_id, args).await?,
        "duplicate_project" => duplicate_project(state, user_id, args).await?,
        "branch_project" => branch_project(state, user_id, args).await?,
        "rollback_project_status" => rollback_project_status(state, user_id, args).await?,
        "get_project_status_history" => get_project_status_history(state, user_id, args).await?,
        "list_chapters" => list_chapters(state, user_id, args).await?,
        "preview_chapter_prompt" => preview_chapter_prompt(state, user_id, args).await?,
        "import_chapter" => import_chapter(state, user_id, args).await?,
        "update_chapter_content" => update_chapter_content(state, user_id, args).await?,
        "get_chapter_shooting_script" => get_chapter_shooting_script(state, user_id, args).await?,
        "get_chapter_shooting_script_unit_progress" => {
            get_chapter_shooting_script_unit_progress(state, user_id, args).await?
        }
        "generate_chapter_shooting_script" => {
            generate_chapter_shooting_script(state, user_id, args).await?
        }
        "generate_next_chapter_shooting_script_unit" => {
            generate_next_chapter_shooting_script_unit(state, user_id, args).await?
        }
        "edit_chapter_shooting_script" => {
            edit_chapter_shooting_script(state, user_id, args).await?
        }
        "generate_chapter_analysis" => generate_chapter_analysis(state, user_id, args).await?,
        "preview_chapter_analysis_request" => {
            preview_chapter_analysis_request(state, user_id, args).await?
        }
        "resume_chapter_analysis_sync" => {
            resume_chapter_analysis_sync(state, user_id, args).await?
        }
        "preview_resume_chapter_analysis_sync_request" => {
            preview_resume_chapter_analysis_sync_request(state, user_id, args).await?
        }
        "list_chapter_scenes" => list_chapter_scenes(state, user_id, args).await?,
        "update_chapter_scene" => update_chapter_scene(state, user_id, args).await?,
        "delete_chapter_scene" => delete_chapter_scene(state, user_id, args).await?,
        "reorder_chapter_scenes" => reorder_chapter_scenes(state, user_id, args).await?,
        "generate_chapter_scene" => generate_chapter_scene(state, user_id, args).await?,
        "preview_chapter_scene_request" => {
            preview_chapter_scene_request(state, user_id, args).await?
        }
        "get_blueprint" => get_blueprint(state, user_id, args).await?,
        "preview_generate_blueprint_request" => {
            preview_generate_blueprint_request(state, user_id, args).await?
        }
        "preview_refine_blueprint_request" => {
            preview_refine_blueprint_request(state, user_id, args).await?
        }
        "list_blueprint_versions" => list_blueprint_versions(state, user_id, args).await?,
        "diff_blueprint_versions" => diff_blueprint_versions(state, user_id, args).await?,
        "restore_blueprint_version" => restore_blueprint_version(state, user_id, args).await?,
        "preview_regenerate_part_outline_request" => {
            preview_regenerate_part_outline_request(state, user_id, args).await?
        }
        "preview_continue_part_outlines_request" => {
            preview_continue_part_outlines_request(state, user_id, args).await?
        }
        "get_generation_progress" => get_generation_progress(state, user_id, args).await?,
        "list_chapter_outlines" => list_chapter_outlines(state, user_id, args).await?,
        "delete_latest_chapter_outlines" => {
            delete_latest_chapter_outlines(state, user_id, args).await?
        }
        "preview_generate_part_chapter_outlines_request" => {
            preview_generate_part_chapter_outlines_request(state, user_id, args).await?
        }
        "preview_batch_generate_part_outlines_request" => {
            preview_batch_generate_part_outlines_request(state, user_id, args).await?
        }
        "preview_apply_beat_mapping_request" => {
            preview_apply_beat_mapping_request(state, user_id, args).await?
        }
        "preview_generate_chapter_outlines_by_count_request" => {
            preview_generate_chapter_outlines_by_count_request(state, user_id, args).await?
        }
        "preview_regenerate_chapter_outline_request" => {
            preview_regenerate_chapter_outline_request(state, user_id, args).await?
        }
        "update_chapter_outline" => update_chapter_outline(state, user_id, args).await?,
        "get_chapter_outlines_pacing" => get_chapter_outlines_pacing(state, user_id, args).await?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}
