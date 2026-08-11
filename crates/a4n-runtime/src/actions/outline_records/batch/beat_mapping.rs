use super::super::{ApplyBeatMappingInput, ensure_part_access, single_request_preview};
use crate::{AppState, actions::request_preview::ModelRequestPreview};
use a4n_core::error::AppError;

pub async fn preview_apply_beat_mapping_request(
    state: &AppState,
    user_id: i64,
    input: ApplyBeatMappingInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(
        state.orchestrator(),
        state.orchestrator(),
        user_id,
        input.project_id,
        input.part_id,
    )
    .await?;
    let part_id = input.part_id;
    let request = state
        .orchestrator()
        .preview_apply_beat_mapping_validated(input)
        .await?;
    single_request_preview(
        state,
        "outline_beat_mapping",
        "节拍映射请求预览",
        "apply_beat_mapping",
        format!("分部 #{part_id} 节拍映射"),
        "outline",
        request,
    )
    .await
}
