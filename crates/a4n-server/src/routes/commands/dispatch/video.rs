use super::super::*;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "list_video_configs" => list_video_configs(state, user_id, args).await?,
        "add_video_config" => add_video_config(state, user_id, args).await?,
        "update_video_config" => update_video_config(state, user_id, args).await?,
        "remove_video_config" => remove_video_config(state, user_id, args).await?,
        "activate_video_config" => activate_video_config(state, user_id, args).await?,
        "test_video_config" => test_video_config(state, user_id, args).await?,
        "detect_video_config_models" => detect_video_config_models(state, user_id, args).await?,
        "list_video_reference_images" => list_video_reference_images(state, user_id, args).await?,
        "list_videos_by_chapter" => list_videos_by_chapter(state, user_id, args).await?,
        "toggle_video_favorite" => toggle_video_favorite(state, user_id, args).await?,
        "update_video" => update_video(state, user_id, args).await?,
        "select_segment_video" => select_segment_video(state, user_id, args).await?,
        "delete_video" => delete_video(state, user_id, args).await?,
        "load_video_playback_data" => load_video_playback_data(state, user_id, args).await?,
        "assemble_chapter_videos" => assemble_chapter_videos(state, user_id, args).await?,
        "load_cached_assembled_chapter_video_preview" => {
            load_cached_assembled_chapter_video_preview(state, user_id, args).await?
        }
        "push_assembled_chapter_video" => {
            push_assembled_chapter_video(state, user_id, args).await?
        }
        "upload_chapter_video" => upload_chapter_video(state, user_id, args).await?,
        "upload_reference_media_video" => {
            upload_reference_media_video(state, user_id, args).await?
        }
        "upload_assembly_insert_video" => {
            upload_assembly_insert_video(state, user_id, args).await?
        }
        "trim_video" => trim_video(state, user_id, args).await?,
        "list_chapter_video_prompts" => list_chapter_video_prompts(state, user_id, args).await?,
        "load_video_segment_plan_draft" => {
            load_video_segment_plan_draft(state, user_id, args).await?
        }
        "update_video_segment_plan_draft_recipe" => {
            update_video_segment_plan_draft_recipe(state, user_id, args).await?
        }
        "update_chapter_video_prompt" => update_chapter_video_prompt(state, user_id, args).await?,
        "optimize_chapter_video_prompt" => {
            optimize_chapter_video_prompt(state, user_id, args).await?
        }
        "delete_chapter_video_prompt" => delete_chapter_video_prompt(state, user_id, args).await?,
        "poll_video_job" => poll_video_job(state, user_id, args).await?,
        "preview_chapter_video_request" => {
            preview_chapter_video_request(state, user_id, args).await?
        }
        "preview_video_provider_request_by_video_id" => {
            preview_video_provider_request_by_video_id(state, user_id, args).await?
        }
        "preview_reference_media_video_request" => {
            preview_reference_media_video_request(state, user_id, args).await?
        }
        "export_video_studio_project_data" => {
            export_video_studio_project_data(state, user_id, args).await?
        }
        "import_video_studio_project_upload" => {
            import_video_studio_project_upload(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
