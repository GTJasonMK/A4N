mod image;
mod llm;
mod video;

use crate::actions::request_preview::ModelRequestPreview;
use a4n_api_contract::canvas::PreviewCanvasNodeRequestInput;
use a4n_canvas_contract::CanvasPreviewNodeApplicationInput;
use a4n_canvas_host_contract::CanvasNodeRequestPreview;

use crate::AppState;

pub async fn preview_canvas_node_request(
    state: &AppState,
    user_id: i64,
    input: PreviewCanvasNodeRequestInput,
) -> Result<ModelRequestPreview, String> {
    let prepared = state
        .canvas_preview_application()
        .prepare_node(CanvasPreviewNodeApplicationInput {
            user_id,
            project_id: input.project_id,
            graph_id: input.graph_id,
            node_id: input.node_id,
        })
        .await
        .map_err(|error| error.to_string())?;
    match prepared {
        CanvasNodeRequestPreview::Image(image) => {
            image::preview_canvas_image_node(state, user_id, image).await
        }
        CanvasNodeRequestPreview::Video(video) => {
            video::preview_canvas_video_node(state, video).await
        }
        CanvasNodeRequestPreview::Llm(request) => {
            llm::preview_canvas_llm_node(state, request).await
        }
    }
}
