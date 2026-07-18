use super::super::references::ChapterProviderReferences;
use super::ChapterProviderImageInputRequest;
use crate::actions::video_generation::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND, media,
    reference_image_provider_input_by_id, reference_image_provider_inputs,
};

pub(super) async fn resolve_provider_image_inputs(
    request: &ChapterProviderImageInputRequest<'_>,
    cliproxy_has_first_frame: bool,
    should_send_reference_images: bool,
    references: &ChapterProviderReferences,
) -> Result<(Vec<String>, Option<String>, Option<String>), String> {
    let mut reference_image_urls = if should_send_reference_images {
        if references.provider_reference_ids_are_authoritative {
            media::reference_image_provider_inputs_by_ids(
                request.state,
                request.user_id,
                request.project_id,
                request.chapter_id,
                references.provider_reference_image_ids.clone(),
                request.image_transport,
            )
            .await?
        } else {
            reference_image_provider_inputs(
                request.state,
                request.user_id,
                request.project_id,
                request.chapter_id,
                request.prompt_reference_image_ids_json,
                request.image_transport,
            )
            .await?
        }
    } else if request.cliproxy_first_frame_mode
        && !cliproxy_has_first_frame
        && !request.storyboard_workflow
    {
        let mut urls = reference_image_provider_inputs(
            request.state,
            request.user_id,
            request.project_id,
            request.chapter_id,
            request.prompt_reference_image_ids_json,
            request.image_transport,
        )
        .await?;
        urls.truncate(1);
        urls
    } else {
        Vec::new()
    };
    if request.cliproxy_first_frame_mode
        && !request.storyboard_workflow
        && !reference_image_urls.is_empty()
    {
        reference_image_urls.truncate(1);
    }
    let first_frame_image_url = if request.cliproxy_reference_mode || request.storyboard_workflow {
        None
    } else {
        reference_image_provider_input_by_id(
            request.state,
            request.user_id,
            request.project_id,
            request.chapter_id,
            references.boundary_frame_ids.first,
            request.image_transport,
        )
        .await?
    };
    let last_frame_image_url = if request.storyboard_workflow
        || request.backend == NEWAPI_VIDEO_BACKEND
        || request.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
    {
        None
    } else {
        reference_image_provider_input_by_id(
            request.state,
            request.user_id,
            request.project_id,
            request.chapter_id,
            references.boundary_frame_ids.last,
            request.image_transport,
        )
        .await?
    };
    Ok((
        reference_image_urls,
        first_frame_image_url,
        last_frame_image_url,
    ))
}
