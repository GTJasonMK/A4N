use super::super::GalgameMediaBindingResult;
use super::{BindGalgameMediaInput, UnbindGalgameMediaInput};
use crate::AppState;

pub async fn bind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    state
        .galgame()
        .bind_galgame_media_for_user_validated(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn unbind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: UnbindGalgameMediaInput,
) -> Result<u64, String> {
    state
        .galgame()
        .unbind_galgame_media_for_user_validated(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}
