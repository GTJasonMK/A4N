use super::{
    CliproxyGrokReferenceMode, ProviderGenerationWorkflow, append_provider_input_note,
    boundary_frame_missing_image_message, cliproxy_grok_reference_mode,
    enforce_cliproxy_video_media_contract, enforce_provider_image_binding_contract,
    ensure_newapi_first_frame_image_supported, first_frame_chain_missing_start_image_message,
    prompt_provider_workflow, prompt_source_prompt_type, provider_image_input_transport,
    reference_ids_with_boundary_frame_images, reference_ids_with_storyboard_image,
    required_video_execution_identity, storyboard_missing_image_message,
};
use crate::AppState;
use crate::actions::video_generation::{
    BoundaryFrameImageIds, CLIPROXY_OPENAI_VIDEO_BACKEND, ChapterVideoProviderRequestContext,
    NEWAPI_VIDEO_BACKEND, PreviewChapterVideoPromptOverrideInput, ProviderImageInputTransport,
    ProviderPromptImageBinding, VideoProviderRequestPurpose, VideoRequestImageMode,
    boundary_frame_image_ids_for_prompt, first_frame_chain_start_image_id_for_prompt, media,
    storyboard_image_id_for_prompt,
};

mod assembly;
mod image_inputs;
mod preparation;
mod references;

use self::assembly::{ChapterProviderRequestAssemblyInput, assemble_chapter_provider_request};
use self::image_inputs::{ChapterProviderImageInputRequest, resolve_chapter_provider_image_inputs};
use self::preparation::prepare_chapter_provider_request;
use self::references::{ChapterProviderReferenceRequest, resolve_chapter_provider_references};

pub(in crate::actions::video_generation) async fn build_chapter_video_provider_request(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    user_id: i64,
    config_id: Option<i64>,
    purpose: VideoProviderRequestPurpose,
    image_mode: VideoRequestImageMode,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ChapterVideoProviderRequestContext, String> {
    let prepared = prepare_chapter_provider_request(
        state,
        project_id,
        chapter_id,
        prompt_id,
        config_id,
        purpose,
        prompt_override,
    )
    .await?;
    let mut provider_input_note = prepared.provider_input_note;
    let prompt = prepared.prompt;
    let cfg = prepared.cfg;

    let prompt_reference_image_ids_json = prompt.reference_image_ids_json.clone();
    let prompt_reference_image_ids =
        media::reference_image_ids(prompt_reference_image_ids_json.as_ref())?;
    let prompt_reference_image_count = prompt_reference_image_ids.len();
    let source_prompt_type = prompt_source_prompt_type(&prompt).map(str::to_string);
    let video_execution_identity = required_video_execution_identity(&prompt)?;
    let provider_workflow = prompt_provider_workflow(&prompt)?;
    let storyboard_workflow = provider_workflow.uses_storyboard();
    let cliproxy_grok_mode = cliproxy_grok_reference_mode(&cfg)?;
    let cliproxy_first_frame_mode = cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && cliproxy_grok_mode == CliproxyGrokReferenceMode::FirstFrame;
    let cliproxy_reference_mode = cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && cliproxy_grok_mode == CliproxyGrokReferenceMode::Reference;
    enforce_provider_image_binding_contract(
        &cfg.backend,
        provider_workflow,
        cliproxy_grok_mode,
        prompt_reference_image_count,
        purpose,
        &mut provider_input_note,
    )?;
    let image_transport = provider_image_input_transport(&cfg.backend)?;
    if image_transport == ProviderImageInputTransport::RemoteTempUpload {
        append_provider_input_note(
            &mut provider_input_note,
            "远端临时媒体上传：生成时会 POST 图片到 A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL，并使用返回的公网 URL；预览省略真实图片数据。"
                .to_string(),
        );
    }

    let references = resolve_chapter_provider_references(
        ChapterProviderReferenceRequest {
            state,
            project_id,
            chapter_id,
            prompt: &prompt,
            prompt_reference_image_ids: &prompt_reference_image_ids,
            prompt_reference_image_ids_json: prompt_reference_image_ids_json.as_ref(),
            provider_workflow,
            cliproxy_reference_mode,
            purpose,
        },
        &mut provider_input_note,
    )
    .await?;
    let image_inputs = resolve_chapter_provider_image_inputs(
        ChapterProviderImageInputRequest {
            state,
            user_id,
            project_id,
            chapter_id,
            backend: &cfg.backend,
            image_mode,
            image_transport,
            cliproxy_first_frame_mode,
            cliproxy_reference_mode,
            storyboard_workflow,
            prompt_reference_image_count,
            prompt_reference_image_ids_json: prompt_reference_image_ids_json.as_ref(),
        },
        &references,
        &mut provider_input_note,
    )
    .await?;

    if cfg.backend == NEWAPI_VIDEO_BACKEND && purpose == VideoProviderRequestPurpose::Generate {
        ensure_newapi_first_frame_image_supported(image_inputs.first_frame_image_url.as_deref())?;
    }
    let first_frame_image_url_available = image_inputs.first_frame_image_url.is_some()
        || (cliproxy_first_frame_mode
            && !storyboard_workflow
            && !image_inputs.reference_image_urls.is_empty());
    let last_frame_image_url_available = image_inputs.last_frame_image_url.is_some();
    enforce_cliproxy_video_media_contract(
        &cfg.backend,
        &cfg.model,
        cliproxy_grok_mode,
        references.provider_reference_image_count,
        first_frame_image_url_available,
        prompt.duration_secs,
        purpose,
        &mut provider_input_note,
    )?;
    let provider_prompt_image_binding = ProviderPromptImageBinding {
        has_first_frame_image: first_frame_image_url_available,
        has_last_frame_image: last_frame_image_url_available,
        reference_image_count: image_inputs.reference_image_urls.len(),
        has_storyboard_reference_image: storyboard_workflow
            && references.storyboard_image_id.is_some(),
    };
    let assembled = assemble_chapter_provider_request(
        ChapterProviderRequestAssemblyInput {
            state,
            prompt: &prompt,
            cfg: &cfg,
            purpose,
            image_binding: provider_prompt_image_binding,
            image_inputs,
        },
        &mut provider_input_note,
    )
    .await?;
    Ok(ChapterVideoProviderRequestContext {
        prompt_reference_image_count,
        provider_reference_image_count: references.provider_reference_image_count,
        prompt,
        cfg,
        request: assembled.request,
        prompt_reference_image_ids: prompt_reference_image_ids_json,
        provider_reference_image_ids: references.provider_reference_image_ids_json,
        prompt_source: assembled.prompt_source,
        workflow_mode: provider_workflow.as_str().to_string(),
        source_prompt_type,
        video_execution_identity,
        boundary_frame_ids: references.boundary_frame_ids,
        first_frame_image_url_available,
        last_frame_image_url_available,
        provider_input_note,
    })
}
