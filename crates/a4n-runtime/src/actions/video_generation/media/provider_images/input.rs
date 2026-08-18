use super::probe::validate_provider_public_image_url;
use super::public_url::image_file_to_provider_media_url;
use super::remote_upload::image_file_to_remote_temp_media_url;
use crate::actions::video_generation::media::types::{
    MAX_REFERENCE_IMAGE_BYTES, MAX_REFERENCE_IMAGE_TOTAL_BYTES, ProviderImageInputTransport,
};
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::{media::image_mime_from_path, models::ai::generated_image};
use a4n_service_protocol::GenerationArtifactRef;
use base64::Engine as _;
use std::path::Path;

pub(in crate::actions::video_generation::media) async fn image_file_to_provider_input(
    data_dir: &Path,
    jwt_secret: Option<&str>,
    user_id: i64,
    media_base_url: Option<&str>,
    image: &generated_image::Model,
    inline_total_bytes: &mut u64,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
    transport: ProviderImageInputTransport,
) -> Result<String, String> {
    let canonical_path = canonicalize_data_file_path(
        data_dir,
        &image.file_path,
        &format!("参考图片 #{}", image.id),
    )?;
    let metadata = std::fs::metadata(&canonical_path)
        .map_err(|e| format!("无法读取参考图片 #{} 元数据：{e}", image.id))?;
    if metadata.len() > MAX_REFERENCE_IMAGE_BYTES {
        return Err(format!(
            "参考图片 #{} 超过 {}MB，无法作为视频参考图",
            image.id,
            MAX_REFERENCE_IMAGE_BYTES / 1024 / 1024
        ));
    }
    let mime = image_mime_from_path(&canonical_path)?;
    match transport {
        ProviderImageInputTransport::PublicUrl => {
            let Some(base) = media_base_url else {
                return Err("public_url 传输缺少公网媒体入口".to_string());
            };
            let secret = jwt_secret.ok_or_else(|| "public_url 传输缺少 JWT secret".to_string())?;
            let url = image_file_to_provider_media_url(secret, user_id, base, image)?;
            validate_provider_public_image_url(&url, &format!("参考图片 #{}", image.id)).await?;
            Ok(url)
        }
        ProviderImageInputTransport::RemoteTempUpload => {
            let uploaded =
                image_file_to_remote_temp_media_url(&canonical_path, user_id, image, mime).await?;
            staging_artifacts.push(uploaded.artifact);
            Ok(uploaded.url)
        }
        ProviderImageInputTransport::InlineDataUrl => {
            let bytes = std::fs::read(&canonical_path)
                .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
            *inline_total_bytes = inline_total_bytes
                .checked_add(metadata.len())
                .ok_or_else(|| "参考图片总大小超过 u64 上限".to_string())?;
            if *inline_total_bytes > MAX_REFERENCE_IMAGE_TOTAL_BYTES {
                return Err(format!(
                    "参考图片总大小超过 {}MB，无法作为视频参考图",
                    MAX_REFERENCE_IMAGE_TOTAL_BYTES / 1024 / 1024
                ));
            }
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            Ok(format!("data:{mime};base64,{encoded}"))
        }
    }
}
