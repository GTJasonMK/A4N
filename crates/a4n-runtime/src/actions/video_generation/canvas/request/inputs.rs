use super::super::super::{
    AppState, ProviderImageInputTransport, VideoRequestImageMode, append_provider_input_note,
    reference_image_preview_placeholder_by_id, reference_image_provider_input_by_id,
};

pub(super) async fn resolve_canvas_image_input(
    state: &AppState,
    input: &super::super::super::GenerateCanvasVideoInput,
    image_id: Option<i64>,
    image_mode: VideoRequestImageMode,
    image_transport: ProviderImageInputTransport,
    provider_input_note: &mut Option<String>,
) -> Result<Option<String>, String> {
    match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            reference_image_provider_input_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_id,
                image_transport,
            )
            .await
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let (url, notes) = reference_image_preview_placeholder_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_id,
            )
            .await?;
            for note in notes {
                append_provider_input_note(provider_input_note, note);
            }
            Ok(url)
        }
    }
}
