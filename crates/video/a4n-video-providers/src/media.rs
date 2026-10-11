use a4n_domain::{ProviderError, media::image_mime_from_path};
use a4n_media::image_upload_format_from_image_format;
use a4n_storage::canonicalize_data_file_path;
use a4n_video_sidecar_contract::{
    MAX_VIDEO_IMAGE_BYTES, VideoGenerationInput, VideoGenerationRequest, VideoMediaSource,
    validate_video_image_artifact,
};
use base64::Engine as _;
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

/// Host requests carry content identities; only provider submission needs bytes.
pub async fn materialize_video_input(
    request: VideoGenerationInput,
    data_dir: Option<PathBuf>,
) -> Result<VideoGenerationRequest, ProviderError> {
    if request.media.artifacts().next().is_none() {
        return materialize_images(request, None);
    }
    let data_dir = data_dir
        .ok_or_else(|| ProviderError::Configuration("本地视频图片引用缺少应用数据目录".into()))?;
    tokio::task::spawn_blocking(move || materialize_images(request, Some(&data_dir)))
        .await
        .map_err(|error| ProviderError::Internal(format!("读取视频图片任务失败：{error}")))?
}

const INLINE_IMAGE_MAX_SIDE: u32 = 1024;
const INLINE_IMAGE_MAX_BYTES: usize = 1024 * 1024;
const INLINE_IMAGE_JPEG_QUALITY: u8 = 85;

fn inline_image_data_url(mime: &str, bytes: &[u8]) -> Result<String, ProviderError> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| {
            ProviderError::InvalidArgument(format!("视频图片格式无法识别：{error}"))
        })?;
    let format = reader
        .format()
        .ok_or_else(|| ProviderError::InvalidArgument("视频图片格式无法识别".into()))?;
    let actual_mime = image_upload_format_from_image_format(format)
        .map(|image_format| image_format.mime_type)
        .ok_or_else(|| ProviderError::InvalidArgument("视频图片格式不支持".into()))?;
    if actual_mime != mime {
        return Err(ProviderError::InvalidArgument(
            "视频图片 MIME 与文件格式不一致".into(),
        ));
    }
    let image = reader
        .decode()
        .map_err(|error| ProviderError::InvalidArgument(format!("视频图片解码失败：{error}")))?;
    let (width, height) = (image.width(), image.height());
    if bytes.len() <= INLINE_IMAGE_MAX_BYTES
        && width <= INLINE_IMAGE_MAX_SIDE
        && height <= INLINE_IMAGE_MAX_SIDE
    {
        return Ok(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ));
    }

    let resized = image.thumbnail(INLINE_IMAGE_MAX_SIDE, INLINE_IMAGE_MAX_SIDE);
    let mut compressed = Vec::new();
    let output_format = if resized.color().has_alpha() {
        image::ImageFormat::Png
    } else {
        image::ImageFormat::Jpeg
    };
    if output_format == image::ImageFormat::Png {
        resized
            .write_to(&mut Cursor::new(&mut compressed), output_format)
            .map_err(|error| {
                ProviderError::InvalidArgument(format!("视频图片压缩失败：{error}"))
            })?;
    } else {
        image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut compressed,
            INLINE_IMAGE_JPEG_QUALITY,
        )
        .encode_image(&image::DynamicImage::ImageRgb8(resized.to_rgb8()))
        .map_err(|error| ProviderError::InvalidArgument(format!("视频图片压缩失败：{error}")))?;
    }
    let output_mime = image_upload_format_from_image_format(output_format)
        .map(|image_format| image_format.mime_type)
        .ok_or_else(|| ProviderError::InvalidArgument("视频图片压缩输出格式不支持".into()))?;
    Ok(format!(
        "data:{output_mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(compressed)
    ))
}

fn materialize_images(
    request: VideoGenerationInput,
    data_dir: Option<&Path>,
) -> Result<VideoGenerationRequest, ProviderError> {
    request
        .media
        .validate_artifacts()
        .map_err(ProviderError::InvalidArgument)?;
    request.try_map_images(|source| match source {
        VideoMediaSource::Url { url } => Ok(url),
        VideoMediaSource::Artifact { artifact } => {
            let relative_path =
                validate_video_image_artifact(&artifact).map_err(ProviderError::InvalidArgument)?;
            let data_dir = data_dir.ok_or_else(|| {
                ProviderError::Configuration("本地视频图片引用缺少应用数据目录".into())
            })?;
            let path = canonicalize_data_file_path(data_dir, relative_path, "视频图片引用")
                .map_err(ProviderError::InvalidArgument)?;
            let project_root = data_dir
                .canonicalize()
                .map_err(|error| ProviderError::InvalidArgument(error.to_string()))?
                .join(format!("projects/{}", artifact.project_id.unwrap()));
            if !path.starts_with(project_root) {
                return Err(ProviderError::InvalidArgument(
                    "视频图片引用不能跳转到其他项目".into(),
                ));
            }
            let mime = image_mime_from_path(&path).map_err(ProviderError::InvalidArgument)?;
            if mime != artifact.content_type {
                return Err(ProviderError::InvalidArgument(
                    "视频图片 MIME 与文件类型不一致".into(),
                ));
            }
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(MAX_VIDEO_IMAGE_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 != artifact.byte_size
                || format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256
            {
                return Err(ProviderError::InvalidArgument(
                    "视频图片内容已变化，请重新准备请求".into(),
                ));
            }
            inline_image_data_url(mime, &bytes)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_generation_artifact_contract::{ArtifactLocation, ArtifactOwnership, ArtifactRef};
    use a4n_video_sidecar_contract::VideoMediaInput;

    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    fn image_request(root: &Path) -> VideoGenerationInput {
        let image = base64::engine::general_purpose::STANDARD
            .decode(PNG)
            .unwrap();
        let path = "projects/1/images/reference.png";
        std::fs::create_dir_all(root.join("projects/1/images")).unwrap();
        std::fs::write(root.join(path), &image).unwrap();
        VideoGenerationInput {
            prompt: "animate".into(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 6,
            fps: 24,
            model: "test-video".into(),
            media: VideoMediaInput::Frames {
                first: VideoMediaSource::Artifact {
                    artifact: ArtifactRef {
                        artifact_id: "generated-image-1".into(),
                        artifact_kind: "generated_image".into(),
                        owner_user_id: Some(7),
                        project_id: Some(1),
                        content_type: "image/png".into(),
                        sha256: format!("{:x}", Sha256::digest(&image)),
                        byte_size: image.len() as u64,
                        location: ArtifactLocation::LocalPath(path.into()),
                        expires_at: None,
                        ownership: ArtifactOwnership::Host,
                    },
                },
                last: None,
                references: Vec::new(),
            },
        }
    }

    #[tokio::test]
    async fn image_references_stay_small_and_materialize_the_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let request = image_request(dir.path());
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.len() < 1024);
        assert!(!json.contains(PNG));
        for backend in a4n_video_provider_contract::VIDEO_PROVIDER_CAPABILITIES {
            let preview = crate::video_provider_request_body_preview(
                backend.backend,
                "https://provider.test/v1",
                &request,
            )
            .unwrap();
            assert!(preview.to_string().len() < 1024);
            assert!(!preview.to_string().contains(PNG));
        }
        let materialized = materialize_video_input(request.clone(), Some(dir.path().into()))
            .await
            .unwrap();
        assert_eq!(
            materialized.media.first_frame(),
            Some(&format!("data:image/png;base64,{PNG}"))
        );
        let path = dir.path().join("projects/1/images/reference.png");
        let mut changed = std::fs::read(&path).unwrap();
        *changed.last_mut().unwrap() ^= 1;
        std::fs::write(path, changed).unwrap();
        assert!(
            materialize_video_input(request, Some(dir.path().into()))
                .await
                .unwrap_err()
                .to_string()
                .contains("内容已变化")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn image_references_cannot_follow_links_outside_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let request = image_request(dir.path());
        let path = dir.path().join("projects/1/images/reference.png");
        std::fs::create_dir_all(dir.path().join("projects/2/images")).unwrap();
        let other = dir.path().join("projects/2/images/reference.png");
        std::fs::rename(&path, &other).unwrap();
        std::os::unix::fs::symlink(other, &path).unwrap();
        let error = materialize_video_input(request, Some(dir.path().into()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("其他项目"));
    }

    #[test]
    fn oversized_inline_images_are_resized_and_reencoded() {
        let image = image::RgbImage::from_pixel(2048, 1024, image::Rgb([20, 40, 60]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        let data_url = inline_image_data_url("image/png", &bytes).unwrap();
        assert!(data_url.starts_with("data:image/jpeg;base64,"));
        let encoded = data_url.split_once(',').unwrap().1;
        let compressed = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let decoded = image::load_from_memory(&compressed).unwrap();
        assert!(decoded.width() <= INLINE_IMAGE_MAX_SIDE);
        assert!(decoded.height() <= INLINE_IMAGE_MAX_SIDE);
    }

    #[test]
    fn small_inline_images_with_valid_headers_but_broken_data_are_rejected() {
        let image = image::RgbImage::from_pixel(2, 2, image::Rgb([20, 40, 60]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let truncated = &bytes[..41]; // PNG and IHDR/IDAT headers are intact; image data is absent.
        assert!(
            image::ImageReader::new(Cursor::new(truncated))
                .with_guessed_format()
                .unwrap()
                .into_dimensions()
                .is_ok()
        );
        assert!(
            inline_image_data_url("image/png", truncated)
                .unwrap_err()
                .to_string()
                .contains("解码失败")
        );
    }
}
