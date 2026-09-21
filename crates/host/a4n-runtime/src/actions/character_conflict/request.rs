use crate::{
    AppState,
    actions::{
        access,
        request_preview::{
            ModelRequestPreview, model_request_preview, traced_chat_request_step_for_state,
        },
    },
};
use a4n_api_contract::character::CharacterConflictCheckInput;
use a4n_character_host_contract::CHARACTER_CONFLICT_KIND;

pub async fn preview_character_conflict_request(
    state: &AppState,
    user_id: i64,
    input: CharacterConflictCheckInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = state;
    let request = state
        .character_conflict_application()
        .preview(input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    let step = traced_chat_request_step_for_state(
        state,
        request,
        "角色一致性检查 LLM 请求",
        CHARACTER_CONFLICT_KIND,
        "角色一致性检查请求缺少 call_key",
    )
    .await?;
    Ok(model_request_preview(
        CHARACTER_CONFLICT_KIND,
        "角色一致性检查 LLM 请求预览",
        vec![step],
    ))
}
