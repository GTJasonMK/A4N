use super::records::validated_reference_images_for_ids;
use crate::actions::video_generation::media::provider_images::{
    image_file_to_provider_input, public_media_base_url,
};
use crate::actions::video_generation::media::types::ProviderImageInputTransport;
use a4n_api_contract::video::VideoRecordReadApplicationPort;
use a4n_domain::image::validate_reference_image_id_list;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::VideoMediaSource;
use std::path::Path;

pub(crate) struct ReferenceImageProviderContext<'a> {
    pub(crate) video: &'a dyn VideoRecordReadApplicationPort,
    pub(crate) data_dir: &'a Path,
    pub(crate) jwt_secret: Option<&'a str>,
    pub(crate) user_id: i64,
    pub(crate) project_id: i64,
    pub(crate) chapter_id: Option<i64>,
    pub(crate) transport: ProviderImageInputTransport,
}

pub(crate) async fn reference_image_provider_inputs_by_ids(
    context: &ReferenceImageProviderContext<'_>,
    ids: Vec<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
) -> Result<Vec<VideoMediaSource>, String> {
    validate_reference_image_id_list(&ids)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(context, ids, staging_artifacts).await
}

pub(crate) async fn reference_image_provider_input_by_id(
    context: &ReferenceImageProviderContext<'_>,
    id: Option<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
) -> Result<Option<VideoMediaSource>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut urls =
        reference_image_provider_inputs_for_ids(context, vec![id], staging_artifacts).await?;
    Ok(urls.pop())
}

async fn reference_image_provider_inputs_for_ids(
    context: &ReferenceImageProviderContext<'_>,
    ids: Vec<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
) -> Result<Vec<VideoMediaSource>, String> {
    let images = validated_reference_images_for_ids(
        context.video,
        context.user_id,
        context.project_id,
        context.chapter_id,
        ids,
    )
    .await?;
    let mut urls = Vec::with_capacity(images.len());
    let media_base_url = match context.transport {
        ProviderImageInputTransport::PublicUrl => Some(public_media_base_url()?),
        ProviderImageInputTransport::InlineDataUrl
        | ProviderImageInputTransport::RemoteTempUpload => None,
    };
    let mut inline_total_bytes = 0_u64;
    for image in images {
        let input = image_file_to_provider_input(
            context.data_dir,
            context.jwt_secret,
            context.user_id,
            media_base_url.as_deref(),
            &image,
            &mut inline_total_bytes,
            staging_artifacts,
            context.transport,
        )
        .await?;
        urls.push(input);
    }
    Ok(urls)
}
