use super::probe::validate_provider_public_image_url;
use super::public_url::image_file_to_provider_media_url;
use super::remote_upload::image_file_to_remote_temp_media_url;
use crate::actions::video_generation::media::types::{
    MAX_REFERENCE_IMAGE_BYTES, MAX_REFERENCE_IMAGE_TOTAL_BYTES, ProviderImageInputTransport,
};
use a4n_api_contract::image::GeneratedImageSummary;
use a4n_domain::media::image_mime_from_path;
use a4n_service_protocol::{
    ArtifactLocation, ArtifactOwnership, ArtifactRef, GenerationArtifactRef,
};
use a4n_storage::{canonicalize_data_file_path, data_dir_relative_file_path};
use a4n_video::VideoMediaSource;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub(in crate::actions::video_generation::media) async fn image_file_to_provider_input(
    data_dir: &Path,
    jwt_secret: Option<&str>,
    user_id: i64,
    media_base_url: Option<&str>,
    image: &GeneratedImageSummary,
    inline_total_bytes: &mut u64,
    staging_artifacts: &mut Vec<GenerationArtifactRef>,
    transport: ProviderImageInputTransport,
) -> Result<VideoMediaSource, String> {
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
            Ok(url.into())
        }
        ProviderImageInputTransport::RemoteTempUpload => {
            let uploaded =
                image_file_to_remote_temp_media_url(&canonical_path, user_id, image, mime).await?;
            staging_artifacts.push(uploaded.artifact);
            Ok(uploaded.url.into())
        }
        ProviderImageInputTransport::InlineDataUrl => {
            let mut digest = Sha256::new();
            let byte_size = std::fs::File::open(&canonical_path)
                .and_then(|file| {
                    std::io::copy(&mut file.take(MAX_REFERENCE_IMAGE_BYTES + 1), &mut digest)
                })
                .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
            if byte_size > MAX_REFERENCE_IMAGE_BYTES {
                return Err(format!("参考图片 #{} 超过单图片大小限制", image.id));
            }
            *inline_total_bytes = inline_total_bytes
                .checked_add(byte_size)
                .ok_or_else(|| "参考图片总大小超过 u64 上限".to_string())?;
            if *inline_total_bytes > MAX_REFERENCE_IMAGE_TOTAL_BYTES {
                return Err(format!(
                    "参考图片总大小超过 {}MB，无法作为视频参考图",
                    MAX_REFERENCE_IMAGE_TOTAL_BYTES / 1024 / 1024
                ));
            }
            Ok(VideoMediaSource::Artifact {
                artifact: ArtifactRef {
                    artifact_id: format!("generated-image-{}", image.id),
                    artifact_kind: "generated_image".into(),
                    owner_user_id: Some(user_id),
                    project_id: Some(image.project_id),
                    content_type: mime.to_string(),
                    sha256: format!("{:x}", digest.finalize()),
                    byte_size,
                    location: ArtifactLocation::LocalPath(data_dir_relative_file_path(
                        data_dir,
                        &canonical_path,
                        "视频图片",
                    )?),
                    expires_at: None,
                    ownership: ArtifactOwnership::Host,
                },
            })
        }
    }
}
