use super::ModelRequestPreview;
use crate::AppState;
use crate::actions::image_generation::{GenerateCanvasImageInput, preview_canvas_image_request};

pub(super) async fn preview_canvas_image_node(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
) -> Result<ModelRequestPreview, String> {
    preview_canvas_image_request(state, user_id, input).await
}
