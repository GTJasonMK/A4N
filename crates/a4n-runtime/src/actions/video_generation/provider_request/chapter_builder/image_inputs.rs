use super::references::ChapterProviderReferences;
use super::{
    AppState, CLIPROXY_OPENAI_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND, ProviderImageInputTransport,
    VideoRequestImageMode, append_provider_input_note,
};
use serde_json::Value;

pub(super) struct ChapterProviderImageInputs {
    pub(super) reference_image_urls: Vec<String>,
    pub(super) first_frame_image_url: Option<String>,
    pub(super) last_frame_image_url: Option<String>,
}

pub(super) struct ChapterProviderImageInputRequest<'a> {
    pub(super) state: &'a AppState,
    pub(super) user_id: i64,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) backend: &'a str,
    pub(super) image_mode: VideoRequestImageMode,
    pub(super) image_transport: ProviderImageInputTransport,
    pub(super) cliproxy_first_frame_mode: bool,
    pub(super) cliproxy_reference_mode: bool,
    pub(super) storyboard_workflow: bool,
    pub(super) prompt_reference_image_count: usize,
    pub(super) prompt_reference_image_ids_json: Option<&'a Value>,
}

pub(super) async fn resolve_chapter_provider_image_inputs(
    request: ChapterProviderImageInputRequest<'_>,
    references: &ChapterProviderReferences,
    provider_input_note: &mut Option<String>,
) -> Result<ChapterProviderImageInputs, String> {
    let cliproxy_has_first_frame = request.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && references.boundary_frame_ids.first.is_some();
    let should_send_reference_images = request.backend != NEWAPI_VIDEO_BACKEND
        && !(request.cliproxy_first_frame_mode
            && (cliproxy_has_first_frame || request.storyboard_workflow));
    let (reference_image_urls, first_frame_image_url, last_frame_image_url) =
        match request.image_mode {
            VideoRequestImageMode::ProviderImageInputs => {
                resolve_provider_image_inputs(
                    &request,
                    cliproxy_has_first_frame,
                    should_send_reference_images,
                    references,
                )
                .await?
            }
            VideoRequestImageMode::PreviewPlaceholders => {
                resolve_preview_image_inputs(
                    &request,
                    cliproxy_has_first_frame,
                    should_send_reference_images,
                    references,
                    provider_input_note,
                )
                .await?
            }
        };
    if request.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && request.cliproxy_first_frame_mode
        && !request.storyboard_workflow
        && request.prompt_reference_image_count > 0
    {
        let note = if references.boundary_frame_ids.first.is_some() {
            format!(
                "CLIProxy/Grok 首帧模式已使用生成首帧作为 input_reference，{} 张提示词参考图不会作为 reference_images 发送。",
                request.prompt_reference_image_count
            )
        } else {
            format!(
                "CLIProxy/Grok 首帧模式会把第一张提示词参考图作为 input_reference 使用；其余 {} 张提示词参考图不会发送。",
                request.prompt_reference_image_count - 1
            )
        };
        append_provider_input_note(provider_input_note, note);
    }
    Ok(ChapterProviderImageInputs {
        reference_image_urls,
        first_frame_image_url,
        last_frame_image_url,
    })
}

mod provider;

use provider::resolve_provider_image_inputs;

mod preview;

use preview::resolve_preview_image_inputs;
