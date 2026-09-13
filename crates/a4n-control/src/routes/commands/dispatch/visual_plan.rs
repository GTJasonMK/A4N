use super::super::{
    ApiError, ServerState, confirm_chapter_visual_plan, discard_chapter_visual_plan_draft,
    edit_chapter_visual_plan_draft, get_chapter_visual_plan, read_chapter_visual_plan,
    save_chapter_visual_plan_draft, validate_chapter_visual_plan,
};
use serde_json::Value;

// Keep these futures out of the project dispatcher: its debug poll stack grows
// with every command branch and can overflow a native worker even on a read.
pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "get_chapter_visual_plan" => get_chapter_visual_plan(state, user_id, args).await?,
        "read_chapter_visual_plan" => read_chapter_visual_plan(state, user_id, args).await?,
        "edit_chapter_visual_plan_draft" => {
            edit_chapter_visual_plan_draft(state, user_id, args).await?
        }
        "validate_chapter_visual_plan" => {
            validate_chapter_visual_plan(state, user_id, args).await?
        }
        "save_chapter_visual_plan_draft" => {
            save_chapter_visual_plan_draft(state, user_id, args).await?
        }
        "confirm_chapter_visual_plan" => confirm_chapter_visual_plan(state, user_id, args).await?,
        "discard_chapter_visual_plan_draft" => {
            discard_chapter_visual_plan_draft(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
