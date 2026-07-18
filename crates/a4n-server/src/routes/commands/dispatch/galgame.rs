use super::super::*;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "get_galgame_script" => get_galgame_script(state, user_id, args).await?,
        "start_galgame_pipeline" => start_galgame_pipeline(state, user_id, args).await?,
        "cancel_galgame_pipeline" => cancel_galgame_pipeline(state, user_id, args).await?,
        "preview_galgame_pipeline_request" => {
            preview_galgame_pipeline_request(state, user_id, args).await?
        }
        "get_galgame_pipeline_status" => get_galgame_pipeline_status(state, user_id, args).await?,
        "update_galgame_line" => update_galgame_line(state, user_id, args).await?,
        "insert_galgame_line" => insert_galgame_line(state, user_id, args).await?,
        "update_galgame_scene" => update_galgame_scene(state, user_id, args).await?,
        "delete_galgame_line" => delete_galgame_line(state, user_id, args).await?,
        "reorder_galgame_lines" => reorder_galgame_lines(state, user_id, args).await?,
        "update_galgame_choices" => update_galgame_choices(state, user_id, args).await?,
        "plan_galgame_assets" => plan_galgame_assets(state, user_id, args).await?,
        "run_galgame_asset_generation" => {
            run_galgame_asset_generation(state, user_id, args).await?
        }
        "bind_galgame_media" => bind_galgame_media(state, user_id, args).await?,
        "unbind_galgame_media" => unbind_galgame_media(state, user_id, args).await?,
        "import_galgame_audio" => import_galgame_audio(state, user_id, args).await?,
        "export_galgame_webgal" => export_galgame_webgal(state, user_id, args).await?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}
