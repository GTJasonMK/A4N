use super::records::validated_reference_images_for_ids;
use crate::actions::video_generation::media::provider_images::{
    image_file_to_provider_input, public_media_base_url,
};
use crate::actions::video_generation::media::types::ProviderImageInputTransport;
use a4n_domain::image::validate_reference_image_id_list;
use a4n_orchestrator::video::VideoRecordApplicationPort;
use a4n_service_protocol::GenerationArtifactRef;
use std::path::Path;

pub(crate) async fn reference_image_provider_inputs_by_ids(
    video: &dyn VideoRecordApplicationPort,
    data_dir: &Path,
    jwt_secret: Option<&str>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    validate_reference_image_id_list(&ids)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(
        video,
        data_dir,
        jwt_secret,
        user_id,
        project_id,
        chapter_id,
        ids,
        staging_artifacts,
        transport,
    )
    .await
}

pub(crate) async fn reference_image_provider_input_by_id(
    video: &dyn VideoRecordApplicationPort,
    data_dir: &Path,
    jwt_secret: Option<&str>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
    transport: ProviderImageInputTransport,
) -> Result<Option<String>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut urls = reference_image_provider_inputs_for_ids(
        video,
        data_dir,
        jwt_secret,
        user_id,
        project_id,
        chapter_id,
        vec![id],
        staging_artifacts,
        transport,
    )
    .await?;
    Ok(urls.pop())
}

async fn reference_image_provider_inputs_for_ids(
    video: &dyn VideoRecordApplicationPort,
    data_dir: &Path,
    jwt_secret: Option<&str>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    let images =
        validated_reference_images_for_ids(video, user_id, project_id, chapter_id, ids).await?;
    let mut urls = Vec::with_capacity(images.len());
    let media_base_url = match transport {
        ProviderImageInputTransport::PublicUrl => Some(public_media_base_url()?),
        ProviderImageInputTransport::InlineDataUrl
        | ProviderImageInputTransport::RemoteTempUpload => None,
    };
    let mut inline_total_bytes = 0_u64;
    for image in images {
        let input = image_file_to_provider_input(
            data_dir,
            jwt_secret,
            user_id,
            media_base_url.as_deref(),
            &image,
            &mut inline_total_bytes,
            staging_artifacts,
            transport,
        )
        .await?;
        urls.push(input);
    }
    Ok(urls)
}
