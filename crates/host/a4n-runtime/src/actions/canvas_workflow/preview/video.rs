use super::ModelRequestPreview;
use crate::AppState;
use crate::actions::video_generation::{GenerateCanvasVideoInput, preview_canvas_video_request};

pub(super) async fn preview_canvas_video_node(
    state: &AppState,
    input: GenerateCanvasVideoInput,
) -> Result<ModelRequestPreview, String> {
    preview_canvas_video_request(state, input).await
}
