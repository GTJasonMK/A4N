use a4n_core::models::{ai::video_gen_config, chapter::chapter_video_prompt};
use a4n_video::VideoGenerationRequest;
use serde_json::Value;

use super::super::{
    build_video_generation_provider_preview_prompt_with_binding_for_state,
    build_video_generation_provider_prompt_with_binding_for_state,
    enforce_provider_prompt_length_contract, provider_specific_negative_prompt,
};
use super::{AppState, ProviderPromptImageBinding, VideoProviderRequestPurpose};
use crate::actions::video_generation::provider_request::chapter_builder::image_inputs::ChapterProviderImageInputs;

pub(super) struct ChapterProviderRequestAssemblyInput<'a> {
    pub(super) state: &'a AppState,
    pub(super) prompt: &'a chapter_video_prompt::Model,
    pub(super) cfg: &'a video_gen_config::Model,
    pub(super) purpose: VideoProviderRequestPurpose,
    pub(super) image_binding: ProviderPromptImageBinding,
    pub(super) image_inputs: ChapterProviderImageInputs,
}

pub(super) struct ChapterProviderRequestAssembly {
    pub(super) request: VideoGenerationRequest,
    pub(super) prompt_source: Option<Value>,
}

pub(super) async fn assemble_chapter_provider_request(
    input: ChapterProviderRequestAssemblyInput<'_>,
    provider_input_note: &mut Option<String>,
) -> Result<ChapterProviderRequestAssembly, String> {
    let prompt_source = input
        .prompt
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .cloned();
    let full_prompt = match input.purpose {
        VideoProviderRequestPurpose::Generate => {
            build_video_generation_provider_prompt_with_binding_for_state(
                input.state,
                input.prompt,
                input.image_binding,
            )
            .await?
        }
        VideoProviderRequestPurpose::Preview => {
            build_video_generation_provider_preview_prompt_with_binding_for_state(
                input.state,
                input.prompt,
                input.image_binding,
            )
            .await?
        }
    };
    let request = VideoGenerationRequest {
        prompt: full_prompt,
        negative_prompt: provider_specific_negative_prompt(input.prompt.negative_prompt.as_deref()),
        width: input.cfg.default_width,
        height: input.cfg.default_height,
        duration_secs: input.prompt.duration_secs,
        fps: input.cfg.default_fps,
        model: input.cfg.model.clone(),
        first_frame_image_url: input.image_inputs.first_frame_image_url,
        last_frame_image_url: input.image_inputs.last_frame_image_url,
        reference_image_urls: input.image_inputs.reference_image_urls,
        reference_media: None,
    };
    enforce_provider_prompt_length_contract(
        &input.cfg.backend,
        &request.prompt,
        input.purpose,
        provider_input_note,
    )?;
    Ok(ChapterProviderRequestAssembly {
        request,
        prompt_source,
    })
}
