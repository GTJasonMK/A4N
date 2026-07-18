use super::ids::{ensure_reference_image_ids_valid, reference_image_ids};
use super::records::validated_reference_images_for_ids;
use crate::AppState;
use crate::actions::video_generation::media::provider_images::{
    image_file_to_provider_input, public_media_base_url,
};
use crate::actions::video_generation::media::types::ProviderImageInputTransport;
use serde_json::Value;

pub(crate) async fn reference_image_provider_inputs(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    value: Option<&Value>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    let ids = reference_image_ids(value)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(state, user_id, project_id, chapter_id, ids, transport)
        .await
}

pub(crate) async fn reference_image_provider_inputs_by_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    ensure_reference_image_ids_valid(&ids)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(state, user_id, project_id, chapter_id, ids, transport)
        .await
}

pub(crate) async fn reference_image_provider_input_by_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Option<String>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut urls = reference_image_provider_inputs_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        vec![id],
        transport,
    )
    .await?;
    Ok(urls.pop())
}

async fn reference_image_provider_inputs_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    let images =
        validated_reference_images_for_ids(state, user_id, project_id, chapter_id, ids).await?;
    let mut urls = Vec::with_capacity(images.len());
    let media_base_url = match transport {
        ProviderImageInputTransport::PublicUrl => Some(public_media_base_url()?),
        ProviderImageInputTransport::InlineDataUrl
        | ProviderImageInputTransport::RemoteTempUpload => None,
    };
    let mut inline_total_bytes = 0_u64;
    for image in images {
        let input = image_file_to_provider_input(
            state,
            user_id,
            media_base_url.as_deref(),
            &image,
            &mut inline_total_bytes,
            transport,
        )
        .await?;
        urls.push(input);
    }
    Ok(urls)
}
