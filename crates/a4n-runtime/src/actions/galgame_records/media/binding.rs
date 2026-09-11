use super::super::GalgameMediaBindingSummary;
use super::{BindGalgameMediaInput, UnbindGalgameMediaInput};
use crate::AppState;

pub async fn bind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingSummary, String> {
    state
        .galgame_record_application()
        .bind_media(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn unbind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: UnbindGalgameMediaInput,
) -> Result<u64, String> {
    state
        .galgame_record_application()
        .unbind_media(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}
