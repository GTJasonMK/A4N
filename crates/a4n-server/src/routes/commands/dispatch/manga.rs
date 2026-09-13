use super::super::{
    ApiError, ServerState, bind_manga_reference_image, cancel_manga_pipeline,
    delete_manga_by_chapter, delete_manga_prompt, get_manga_pipeline_status,
    list_manga_page_prompts, list_manga_pipeline_statuses, optimize_manga_prompt,
    preview_resume_manga_pipeline_request, preview_start_manga_pipeline_request,
    resume_manga_pipeline, start_manga_pipeline, update_manga_prompt,
};
use serde_json::Value;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "list_manga_page_prompts" => list_manga_page_prompts(state, user_id, args).await?,
        "bind_manga_reference_image" => bind_manga_reference_image(state, user_id, args).await?,
        "update_manga_prompt" => update_manga_prompt(state, user_id, args).await?,
        "optimize_manga_prompt" => optimize_manga_prompt(state, user_id, args).await?,
        "delete_manga_prompt" => delete_manga_prompt(state, user_id, args).await?,
        "delete_manga_by_chapter" => delete_manga_by_chapter(state, user_id, args).await?,
        "get_manga_pipeline_status" => get_manga_pipeline_status(state, user_id, args).await?,
        "list_manga_pipeline_statuses" => {
            list_manga_pipeline_statuses(state, user_id, args).await?
        }
        "cancel_manga_pipeline" => cancel_manga_pipeline(state, user_id, args).await?,
        "start_manga_pipeline" => start_manga_pipeline(state, user_id, args).await?,
        "resume_manga_pipeline" => resume_manga_pipeline(state, user_id, args).await?,
        "preview_start_manga_pipeline_request" => {
            preview_start_manga_pipeline_request(state, user_id, args).await?
        }
        "preview_resume_manga_pipeline_request" => {
            preview_resume_manga_pipeline_request(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
