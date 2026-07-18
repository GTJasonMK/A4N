use super::probe::validate_provider_public_image_url;
use super::rules::truncate_error_body;
use crate::actions::video_generation::media::reference_images::remote_temp_upload_config;
use a4n_core::models::ai::generated_image;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize)]
struct RemoteTempMediaUploadRequest {
    file_name: String,
    mime_type: String,
    data_base64: String,
}

#[derive(Debug, Deserialize)]
struct RemoteTempMediaUploadResponse {
    url: String,
}

pub(super) async fn image_file_to_remote_temp_media_url(
    canonical_path: &Path,
    image: &generated_image::Model,
    mime: &str,
) -> Result<String, String> {
    let (upload_url, token) = remote_temp_upload_config()?;
    let bytes = std::fs::read(canonical_path)
        .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("video-input.png")
        .to_string();
    let payload = RemoteTempMediaUploadRequest {
        file_name,
        mime_type: mime.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    };
    let response = reqwest::Client::new()
        .post(upload_url.trim())
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("上传远端临时媒体失败：{err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("读取远端临时媒体响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!(
            "上传远端临时媒体失败：HTTP {}：{}",
            status.as_u16(),
            truncate_error_body(&text)
        ));
    }
    let parsed: RemoteTempMediaUploadResponse = serde_json::from_str(&text).map_err(|err| {
        format!(
            "解析远端临时媒体响应失败：{err}；响应：{}",
            truncate_error_body(&text)
        )
    })?;
    let url = parsed.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("远端临时媒体响应 url 必须以 http:// 或 https:// 开头".to_string());
    }
    validate_provider_public_image_url(url, &format!("远端临时媒体参考图片 #{}", image.id)).await?;
    Ok(url.to_string())
}
