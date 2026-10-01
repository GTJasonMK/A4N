use a4n_error::AppError;
use a4n_video::VideoGenerationResult;
#[cfg(feature = "embedded-media-providers")]
use a4n_video::video_provider_capabilities;
use a4n_video_host_contract::{VideoProviderRequestBody, VideoProviderRequestInput};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{AppState, actions::video_generation::common::video_sidecar_root_project_id};

use super::provider_host::host_error;

pub(super) async fn provider_request_body(
    state: &AppState,
    input: VideoProviderRequestInput,
) -> Result<VideoProviderRequestBody, AppError> {
    if state.runtime_profile().uses_sidecars() {
        let root_project_id = video_sidecar_root_project_id(
            state.runtime_profile(),
            state.project_access_application(),
            input.project_id,
        )
        .await
        .map_err(host_error)?;
        let request = crate::sidecar_clients::preview_video_provider_request(
            state.capabilities().video.clone(),
            &input.config,
            input.request,
            Some(input.project_id),
            root_project_id,
        )
        .await
        .map_err(host_error)?;
        return Ok(VideoProviderRequestBody {
            request,
            source_label: "由 video sidecar 生成真实 provider 请求体".to_string(),
        });
    }

    #[cfg(feature = "embedded-media-providers")]
    {
        let capabilities = video_provider_capabilities(&input.config.backend).ok_or_else(|| {
            host_error(format!(
                "未知视频 backend={}，无法展示真实 provider 请求体",
                input.config.backend
            ))
        })?;
        let request = a4n_video_providers::video_provider_request_body_preview(
            &input.config.backend,
            input
                .config
                .base_url
                .as_deref()
                .ok_or_else(|| host_error("视频生成配置缺少 base_url".to_string()))?,
            &input.request,
        )
        .map_err(|error| host_error(error.to_string()))?;
        return Ok(VideoProviderRequestBody {
            request,
            source_label: format!(
                "由 embedded {} provider 生成真实请求体",
                capabilities.display_name
            ),
        });
    }

    #[cfg(not(feature = "embedded-media-providers"))]
    Err(host_error(format!(
        "视频 provider 请求不可用：backend={}；当前 runtime 既未启用 sidecar profile，也未编译 embedded-media-providers",
        input.config.backend
    )))
}

pub(super) async fn upload_reference_media_input(
    owner_user_id: i64,
    project_id: i64,
    source_media_id: i64,
    source_media_kind: &str,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<(String, a4n_service_protocol::GenerationArtifactRef), AppError> {
    let uploaded = super::media::media_bytes_to_remote_temp_media_url(
        a4n_api_contract::temp_media::TempMediaUploadRequest {
            owner_user_id,
            project_id,
            source_media_kind: source_media_kind.to_string(),
            source_media_id,
            file_name: file_name.to_string(),
            mime_type: mime_type.to_string(),
        },
        bytes.to_vec(),
    )
    .await
    .map_err(host_error)?;
    Ok((uploaded.url, uploaded.artifact))
}

pub(super) fn completed_video_artifact_metadata(result: &VideoGenerationResult) -> Value {
    json!({
        "provider_job_id": result.provider_job_id,
        "artifact": {
            "format": result.format,
            "byte_size": result.bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&result.bytes)),
        },
    })
}
