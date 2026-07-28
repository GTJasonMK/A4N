use super::super::*;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "list_image_configs" => list_image_configs(state, user_id, args).await?,
        "add_image_config" => add_image_config(state, user_id, args).await?,
        "update_image_config" => update_image_config(state, user_id, args).await?,
        "remove_image_config" => remove_image_config(state, user_id, args).await?,
        "activate_image_config" => activate_image_config(state, user_id, args).await?,
        "test_image_config" => test_image_config(state, user_id, args).await?,
        "preview_test_image_config_request" => {
            preview_test_image_config_request(state, user_id, args).await?
        }
        "detect_image_config_models" => detect_image_config_models(state, user_id, args).await?,
        "preview_scene_image_request" => preview_scene_image_request(state, user_id, args).await?,
        "preview_page_image_request" => preview_page_image_request(state, user_id, args).await?,
        "preview_batch_images_request" => {
            preview_batch_images_request(state, user_id, args).await?
        }
        "list_images_by_project" => list_images_by_project(state, user_id, args).await?,
        "list_images_by_chapter" => list_images_by_chapter(state, user_id, args).await?,
        "list_profile_materials" => list_profile_materials(state, user_id, args).await?,
        "upload_profile_material_image" => {
            upload_profile_material_image(state, user_id, args).await?
        }
        "toggle_image_favorite" => toggle_image_favorite(state, user_id, args).await?,
        "update_image" => update_image(state, user_id, args).await?,
        "upload_image" => upload_image(state, user_id, args).await?,
        "delete_image" => delete_image(state, user_id, args).await?,
        "score_image" => score_image(state, user_id, args).await?,
        "list_object_profiles" => list_object_profiles(state, user_id, args).await?,
        "create_object_profile" => create_object_profile(state, user_id, args).await?,
        "update_object_profile" => update_object_profile(state, user_id, args).await?,
        "delete_object_profile" => delete_object_profile(state, user_id, args).await?,
        "generate_object_profile_image_prompt" => {
            generate_object_profile_image_prompt(state, user_id, args).await?
        }
        "preview_object_profile_image_request" => {
            preview_object_profile_image_request(state, user_id, args).await?
        }
        "list_scene_profiles" => list_scene_profiles(state, user_id, args).await?,
        "create_scene_profile" => create_scene_profile(state, user_id, args).await?,
        "update_scene_profile" => update_scene_profile(state, user_id, args).await?,
        "delete_scene_profile" => delete_scene_profile(state, user_id, args).await?,
        "generate_scene_profile_image_prompt" => {
            generate_scene_profile_image_prompt(state, user_id, args).await?
        }
        "sync_scene_profiles" => sync_scene_profiles(state, user_id, args).await?,
        "get_character_arc_status" => get_character_arc_status(state, user_id, args).await?,
        "get_character_voice_fingerprint" => {
            get_character_voice_fingerprint(state, user_id, args).await?
        }
        "discover_character_profiles" => discover_character_profiles(state, user_id, args).await?,
        "preview_discover_character_profiles_request" => {
            preview_discover_character_profiles_request(state, user_id, args).await?
        }
        "sync_all_character_profiles" => sync_all_character_profiles(state, user_id, args).await?,
        "preview_sync_all_character_profiles_request" => {
            preview_sync_all_character_profiles_request(state, user_id, args).await?
        }
        "sync_character_from_chapters" => {
            sync_character_from_chapters(state, user_id, args).await?
        }
        "preview_sync_character_from_chapters_request" => {
            preview_sync_character_from_chapters_request(state, user_id, args).await?
        }
        "check_character_conflict" => check_character_conflict(state, user_id, args).await?,
        "preview_check_character_conflict_request" => {
            preview_check_character_conflict_request(state, user_id, args).await?
        }
        "export_characters_json_data" => export_characters_json_data(state, user_id, args).await?,
        "list_portraits_by_project" => list_portraits_by_project(state, user_id, args).await?,
        "get_primary_portrait" => get_primary_portrait(state, user_id, args).await?,
        "set_primary_portrait" => set_primary_portrait(state, user_id, args).await?,
        "upload_portrait" => upload_portrait(state, user_id, args).await?,
        "generate_portrait_prompt" => generate_portrait_prompt(state, user_id, args).await?,
        "preview_generate_portrait_prompt_request" => {
            preview_generate_portrait_prompt_request(state, user_id, args).await?
        }
        "preview_generate_portrait_request" => {
            preview_generate_portrait_request(state, user_id, args).await?
        }
        "preview_regenerate_portrait_request" => {
            preview_regenerate_portrait_request(state, user_id, args).await?
        }
        "preview_generate_missing_portraits_request" => {
            preview_generate_missing_portraits_request(state, user_id, args).await?
        }
        "get_project_asset_dependency_graph" => {
            get_project_asset_dependency_graph(state, user_id, args).await?
        }
        "get_project_asset_dependency_graph_node_detail" => {
            get_project_asset_dependency_graph_node_detail(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
