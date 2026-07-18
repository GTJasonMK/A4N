use super::super::super::*;
use super::super::references::ChapterProviderReferences;
use super::ChapterProviderImageInputRequest;

pub(super) async fn resolve_preview_image_inputs(
    request: &ChapterProviderImageInputRequest<'_>,
    cliproxy_has_first_frame: bool,
    should_send_reference_images: bool,
    references: &ChapterProviderReferences,
    provider_input_note: &mut Option<String>,
) -> Result<(Vec<String>, Option<String>, Option<String>), String> {
    let reference_image_urls = if should_send_reference_images
        || (request.cliproxy_first_frame_mode
            && !cliproxy_has_first_frame
            && !request.storyboard_workflow)
    {
        let placeholders = if references.provider_reference_ids_are_authoritative {
            media::reference_image_preview_placeholders_by_ids(
                request.state,
                request.user_id,
                request.project_id,
                request.chapter_id,
                references.provider_reference_image_ids.clone(),
            )
            .await?
        } else {
            reference_image_preview_placeholders(
                request.state,
                request.user_id,
                request.project_id,
                request.chapter_id,
                request.prompt_reference_image_ids_json,
            )
            .await?
        };
        append_provider_input_notes(provider_input_note, placeholders.notes);
        if request.cliproxy_first_frame_mode && !request.storyboard_workflow {
            placeholders.urls.into_iter().take(1).collect()
        } else {
            placeholders.urls
        }
    } else {
        Vec::new()
    };
    let first_frame_image_url = if request.cliproxy_reference_mode || request.storyboard_workflow {
        None
    } else {
        let (url, notes) = reference_image_preview_placeholder_by_id(
            request.state,
            request.user_id,
            request.project_id,
            request.chapter_id,
            references.boundary_frame_ids.first,
        )
        .await?;
        append_provider_input_notes(provider_input_note, notes);
        url
    };
    let last_frame_image_url = if request.storyboard_workflow
        || request.backend == NEWAPI_VIDEO_BACKEND
        || request.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
    {
        None
    } else {
        let (url, notes) = reference_image_preview_placeholder_by_id(
            request.state,
            request.user_id,
            request.project_id,
            request.chapter_id,
            references.boundary_frame_ids.last,
        )
        .await?;
        append_provider_input_notes(provider_input_note, notes);
        url
    };
    Ok((
        reference_image_urls,
        first_frame_image_url,
        last_frame_image_url,
    ))
}
