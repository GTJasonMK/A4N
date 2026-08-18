use super::probe::validate_provider_public_image_url;
use crate::actions::video_generation::media::reference_images::remote_temp_upload_config;
use a4n_api_contract::temp_media::{
    TEMP_MEDIA_MAX_TTL_SECONDS, TempMediaDeleteRequest, TempMediaUploadRequest,
    TempMediaUploadResponse,
};
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::video::truncate_error_body;
use a4n_service_protocol::GenerationArtifactRef;
use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteTempMediaDeleteOutcome {
    Deleted,
    NotFound,
}

pub(crate) struct RemoteTempMediaUpload {
    pub url: String,
    pub artifact: GenerationArtifactRef,
}

pub(crate) async fn delete_remote_temp_media_artifact(
    artifact_id: &str,
    sha256: &str,
) -> Result<RemoteTempMediaDeleteOutcome, String> {
    let artifact_id = artifact_id.trim();
    if !valid_remote_temp_artifact_id(artifact_id) {
        return Err("远端临时媒体 artifact_id 无效".to_string());
    }
    let sha256 = sha256.trim().to_ascii_lowercase();
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("远端临时媒体 sha256 无效".to_string());
    }
    let (upload_url, token) = crate::actions::video_generation::media::remote_temp_upload_config()?;
    let url = remote_temp_media_delete_url(&upload_url, artifact_id)?;
    let response = reqwest::Client::new()
        .delete(url)
        .bearer_auth(token)
        .json(&TempMediaDeleteRequest { sha256 })
        .send()
        .await
        .map_err(|error| format!("删除远端临时媒体失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("读取远端临时媒体删除响应失败：{error}"))?;
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(RemoteTempMediaDeleteOutcome::Deleted);
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(RemoteTempMediaDeleteOutcome::NotFound);
    }
    Err(format!(
        "删除远端临时媒体失败：HTTP {}：{}",
        status.as_u16(),
        truncate_error_body(&body)
    ))
}

fn remote_temp_media_delete_url(
    upload_url: &str,
    artifact_id: &str,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(upload_url)
        .map_err(|error| format!("远端临时媒体删除地址无效：{error}"))?;
    url.path_segments_mut()
        .map_err(|_| "远端临时媒体删除地址不能包含不可分段路径".to_string())?
        .push(artifact_id);
    Ok(url)
}

pub(super) async fn image_file_to_remote_temp_media_url(
    canonical_path: &Path,
    owner_user_id: i64,
    image: &generated_image::Model,
    mime: &str,
) -> Result<RemoteTempMediaUpload, String> {
    let bytes = std::fs::read(canonical_path)
        .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("参考图片 #{} 路径缺少有效 UTF-8 文件名", image.id))?;
    let uploaded = media_bytes_to_remote_temp_media_url(
        owner_user_id,
        image.project_id,
        "generated_image",
        image.id,
        &file_name,
        mime,
        &bytes,
    )
    .await?;
    validate_provider_public_image_url(
        &uploaded.url,
        &format!("远端临时媒体参考图片 #{}", image.id),
    )
    .await?;
    Ok(uploaded)
}

pub(crate) async fn media_bytes_to_remote_temp_media_url(
    owner_user_id: i64,
    project_id: i64,
    source_media_kind: &str,
    source_media_id: i64,
    file_name: &str,
    mime: &str,
    bytes: &[u8],
) -> Result<RemoteTempMediaUpload, String> {
    let (upload_url, token) = remote_temp_upload_config()?;
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    let payload = TempMediaUploadRequest {
        owner_user_id,
        project_id,
        source_media_kind: source_media_kind.to_string(),
        source_media_id,
        file_name: file_name.to_string(),
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
    let parsed: TempMediaUploadResponse = serde_json::from_str(&text).map_err(|err| {
        format!(
            "解析远端临时媒体响应失败：{err}；响应：{}",
            truncate_error_body(&text)
        )
    })?;
    let (url, expires_at_ms) = validate_remote_temp_media_response(
        &parsed,
        owner_user_id,
        project_id,
        source_media_kind,
        source_media_id,
        mime,
        &sha256,
        bytes.len(),
        Utc::now(),
    )?;
    tracing::info!(
        artifact_id = parsed.artifact_id.trim(),
        owner_user_id,
        project_id,
        source_media_kind,
        source_media_id,
        expires_at = parsed.expires_at,
        "远端临时媒体上传完成"
    );
    Ok(RemoteTempMediaUpload {
        url: url.to_string(),
        artifact: GenerationArtifactRef {
            artifact_id: parsed.artifact_id.trim().to_string(),
            artifact_kind: "remote_temp_media".to_string(),
            content_type: parsed.content_type.clone(),
            sha256: sha256.clone(),
            byte_size: parsed.byte_size,
            revision: sha256,
            temporary: true,
            expires_at_ms: Some(expires_at_ms),
        },
    })
}

fn validate_remote_temp_media_response<'a>(
    response: &'a TempMediaUploadResponse,
    owner_user_id: i64,
    project_id: i64,
    source_media_kind: &str,
    source_media_id: i64,
    mime_type: &str,
    sha256: &str,
    byte_size: usize,
    now: DateTime<Utc>,
) -> Result<(&'a str, i64), String> {
    let artifact_id = response.artifact_id.trim();
    if !valid_remote_temp_artifact_id(artifact_id) {
        return Err("远端临时媒体响应 artifact_id 无效".to_string());
    }
    if response.owner_user_id != owner_user_id
        || response.project_id != project_id
        || response.source_media_kind != source_media_kind
        || response.source_media_id != source_media_id
    {
        return Err("远端临时媒体响应 owner/project/source 不匹配".to_string());
    }
    if !response.content_type.eq_ignore_ascii_case(mime_type) {
        return Err("远端临时媒体响应 content_type 与请求 MIME 不一致".to_string());
    }
    let expected_byte_size =
        u64::try_from(byte_size).map_err(|_| "本地临时媒体大小超出 u64 范围".to_string())?;
    if response.byte_size != expected_byte_size {
        return Err("远端临时媒体响应 byte_size 与上传内容不一致".to_string());
    }
    if response.sha256.len() != 64
        || !response.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !response.sha256.eq_ignore_ascii_case(sha256)
    {
        return Err("远端临时媒体响应 sha256 与上传内容不一致".to_string());
    }

    let url = response.url.trim();
    let parsed_url =
        reqwest::Url::parse(url).map_err(|error| format!("远端临时媒体响应 url 无效：{error}"))?;
    if parsed_url.scheme() != "https" || parsed_url.host_str().is_none() {
        return Err("远端临时媒体响应 url 必须是包含主机名的 https:// URL".to_string());
    }
    if !parsed_url.username().is_empty()
        || parsed_url.password().is_some()
        || parsed_url.fragment().is_some()
    {
        return Err("远端临时媒体响应 url 不能包含凭证或 fragment".to_string());
    }
    let expires_at = DateTime::parse_from_rfc3339(&response.expires_at)
        .map_err(|error| format!("远端临时媒体响应 expires_at 无效：{error}"))?
        .with_timezone(&Utc);
    if expires_at <= now {
        return Err("远端临时媒体响应 expires_at 必须晚于当前时间".to_string());
    }
    if expires_at > now + Duration::seconds(TEMP_MEDIA_MAX_TTL_SECONDS) {
        return Err(format!(
            "远端临时媒体 TTL 不能超过 {TEMP_MEDIA_MAX_TTL_SECONDS} 秒"
        ));
    }
    Ok((url, expires_at.timestamp_millis()))
}

fn valid_remote_temp_artifact_id(artifact_id: &str) -> bool {
    artifact_id.len() == 32 && artifact_id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> TempMediaUploadResponse {
        TempMediaUploadResponse {
            artifact_id: "0123456789abcdef0123456789abcdef".into(),
            url: "https://media.example.test/a.png".into(),
            expires_at: "2026-07-23T11:59:59Z".into(),
            owner_user_id: 7,
            project_id: 11,
            source_media_kind: "generated_image".into(),
            source_media_id: 13,
            content_type: "image/png".into(),
            sha256: "a".repeat(64),
            byte_size: 3,
        }
    }

    #[test]
    fn remote_temp_media_requires_https_and_bounded_ttl() {
        let now = DateTime::parse_from_rfc3339("2026-07-22T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            validate_remote_temp_media_response(
                &response(),
                7,
                11,
                "generated_image",
                13,
                "image/png",
                &"a".repeat(64),
                3,
                now
            )
            .unwrap(),
            ("https://media.example.test/a.png", 1_784_807_999_000)
        );
        let mut invalid = response();
        invalid.url = "http://media.example.test/a.png".into();
        assert!(
            validate_remote_temp_media_response(
                &invalid,
                7,
                11,
                "generated_image",
                13,
                "image/png",
                &"a".repeat(64),
                3,
                now
            )
            .is_err()
        );
        let mut invalid = response();
        invalid.expires_at = "2026-07-24T12:00:00Z".into();
        assert!(
            validate_remote_temp_media_response(
                &invalid,
                7,
                11,
                "generated_image",
                13,
                "image/png",
                &"a".repeat(64),
                3,
                now
            )
            .is_err()
        );
    }

    #[test]
    fn remote_temp_media_rejects_mismatched_audit_metadata() {
        let now = Utc::now();
        let mut response = response();
        response.project_id = 99;
        assert!(
            validate_remote_temp_media_response(
                &response,
                7,
                11,
                "generated_image",
                13,
                "image/png",
                &"a".repeat(64),
                3,
                now,
            )
            .is_err()
        );
    }

    #[test]
    fn remote_temp_media_delete_url_keeps_path_boundary() {
        let url = remote_temp_media_delete_url(
            "https://media.example.test/api/temp-media?scope=video",
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://media.example.test/api/temp-media/0123456789abcdef0123456789abcdef?scope=video"
        );
    }

    #[test]
    fn delete_remote_temp_media_rejects_invalid_artifact_identity_before_network() {
        let invalid_id = delete_remote_temp_media_artifact_blocking("not-hex", &"a".repeat(64));
        assert!(
            invalid_id
                .expect_err("invalid artifact_id must fail before network")
                .contains("artifact_id")
        );

        let invalid_sha = delete_remote_temp_media_artifact_blocking(
            "0123456789abcdef0123456789abcdef",
            "not-a-sha256",
        );
        assert!(
            invalid_sha
                .expect_err("invalid sha256 must fail before network")
                .contains("sha256")
        );
    }

    #[tokio::test]
    async fn delete_remote_temp_media_maps_http_success_and_failure() {
        let _guard = remote_temp_env_lock();
        let artifact_id = "0123456789abcdef0123456789abcdef";
        let sha = "b".repeat(64);

        let deleted = with_remote_delete_server(204, async {
            delete_remote_temp_media_artifact(artifact_id, &sha).await
        })
        .await
        .expect("HTTP 204 should map to Deleted");
        assert_eq!(deleted, RemoteTempMediaDeleteOutcome::Deleted);

        let missing = with_remote_delete_server(404, async {
            delete_remote_temp_media_artifact(artifact_id, &sha).await
        })
        .await
        .expect("HTTP 404 should map to NotFound");
        assert_eq!(missing, RemoteTempMediaDeleteOutcome::NotFound);

        let failed = with_remote_delete_server(500, async {
            delete_remote_temp_media_artifact(artifact_id, &sha).await
        })
        .await
        .expect_err("HTTP 500 must surface as explicit failure");
        assert!(failed.contains("删除远端临时媒体失败"));
        assert!(failed.contains("500"));
    }

    fn delete_remote_temp_media_artifact_blocking(
        artifact_id: &str,
        sha256: &str,
    ) -> Result<RemoteTempMediaDeleteOutcome, String> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(delete_remote_temp_media_artifact(artifact_id, sha256))
    }

    fn remote_temp_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    async fn with_remote_delete_server<F, T>(status: u16, work: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind remote delete fixture");
        let addr = listener.local_addr().expect("fixture addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept delete request");
            let mut buf = vec![0_u8; 4096];
            let _ = socket.read(&mut buf).await;
            let body = if status == 500 { "upstream failed" } else { "" };
            let response = format!(
                "HTTP/1.1 {status} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                if status == 204 {
                    "No Content"
                } else if status == 404 {
                    "Not Found"
                } else {
                    "Internal Server Error"
                },
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write delete response");
        });

        use crate::actions::video_generation::media::types::{
            REMOTE_TEMP_UPLOAD_TOKEN_ENV, REMOTE_TEMP_UPLOAD_URL_ENV,
        };

        let previous_url = std::env::var(REMOTE_TEMP_UPLOAD_URL_ENV).ok();
        let previous_token = std::env::var(REMOTE_TEMP_UPLOAD_TOKEN_ENV).ok();
        // SAFETY: tests hold remote_temp_env_lock and restore env after the fixture.
        unsafe {
            std::env::set_var(
                REMOTE_TEMP_UPLOAD_URL_ENV,
                format!("http://{addr}/temp-media"),
            );
            std::env::set_var(REMOTE_TEMP_UPLOAD_TOKEN_ENV, "process-remote-temp-token");
        }

        let result = work.await;

        // SAFETY: paired with set_var above under the same mutex.
        unsafe {
            match previous_url {
                Some(value) => std::env::set_var(REMOTE_TEMP_UPLOAD_URL_ENV, value),
                None => std::env::remove_var(REMOTE_TEMP_UPLOAD_URL_ENV),
            }
            match previous_token {
                Some(value) => std::env::set_var(REMOTE_TEMP_UPLOAD_TOKEN_ENV, value),
                None => std::env::remove_var(REMOTE_TEMP_UPLOAD_TOKEN_ENV),
            }
        }
        server.await.expect("delete fixture server");
        result
    }
}
