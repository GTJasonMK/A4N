use super::provider::CliProxyOpenAiVideoProvider;
use super::response::*;
use a4n_video::{VideoJobDeleteOutcome, VideoProvider};
use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "animate".into(),
        negative_prompt: Some("noise".into()),
        width: 1280,
        height: 720,
        duration_secs: 5,
        fps: 24,
        model: "grok-imagine-video".into(),
        media: VideoMediaInput::from_images(
            Some("https://example.test/first.png".into()),
            None,
            Vec::new(),
        )
        .unwrap(),
    }
}

#[test]
fn openai_route_uses_wrapper_fields_for_references() {
    let mut req = request();
    req.media =
        VideoMediaInput::from_images(None, None, vec!["https://example.test/ref.png".into()])
            .unwrap();

    let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();
    assert_eq!(body["model"], "grok-imagine-video");
    assert_eq!(body["seconds"], "5");
    assert_eq!(body["size"], "1280x720");
    assert_eq!(body["aspect_ratio"], "16:9");
    assert_eq!(
        body["reference_images"][0]["url"],
        "https://example.test/ref.png"
    );
    assert!(body.get("duration_secs").is_none());
    assert!(body.get("width").is_none());
    assert!(body.get("height").is_none());
    assert!(body.get("fps").is_none());
    assert!(body.get("last_frame_image_url").is_none());
    assert!(body.get("input_reference").is_none());
}

#[test]
fn native_route_uses_native_text_only_fields() {
    let mut req = request();
    req.media = VideoMediaInput::None;

    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/v1",
    )
    .unwrap();

    assert_eq!(body["duration"], 5);
    assert!(body.get("seconds").is_none());
    assert!(body.get("size").is_none());
    assert_eq!(body["aspect_ratio"], "16:9");
    assert_eq!(body["resolution"], "720p");
    assert!(body.get("image").is_none());
    assert!(body.get("reference_images").is_none());
}

#[test]
fn full_hd_request_uses_1080p_resolution() {
    let mut req = request();
    req.width = 1920;
    req.height = 1080;
    req.media = VideoMediaInput::None;

    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/v1",
    )
    .unwrap();

    assert_eq!(body["aspect_ratio"], "16:9");
    assert_eq!(body["resolution"], "1080p");
}

#[test]
fn square_full_hd_quality_uses_1080p_resolution() {
    let mut req = request();
    req.width = 1080;
    req.height = 1080;
    req.media = VideoMediaInput::None;

    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/v1",
    )
    .unwrap();

    assert_eq!(body["aspect_ratio"], "1:1");
    assert_eq!(body["resolution"], "1080p");
}

#[test]
fn native_route_sends_inline_first_frame_as_image() {
    let mut req = request();
    req.media =
        VideoMediaInput::from_images(Some("data:image/png;base64,abc".into()), None, Vec::new())
            .unwrap();
    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/v1",
    )
    .unwrap();
    assert_eq!(body["image"]["url"], "data:image/png;base64,abc");
    assert!(body.get("input_reference").is_none());
}

#[test]
fn tail_frame_is_rejected() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(
        Some("https://example.test/first.png".into()),
        Some("https://example.test/last.png".into()),
        Vec::new(),
    )
    .unwrap();

    let err = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/openai/v1",
    )
    .unwrap_err();

    assert!(err.to_string().contains("没有尾帧输入字段"));
}

#[test]
fn openai_route_sends_inline_first_frame_as_input_reference() {
    let mut req = request();
    req.media =
        VideoMediaInput::from_images(Some("data:image/png;base64,abc".into()), None, Vec::new())
            .unwrap();
    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/openai/v1",
    )
    .unwrap();
    assert_eq!(
        body["input_reference"]["image_url"],
        "data:image/png;base64,abc"
    );
    assert!(body.get("image").is_none());
}

#[test]
fn request_body_sends_references_only_without_first_frame() {
    let mut req = request();
    req.media =
        VideoMediaInput::from_images(None, None, vec!["https://example.test/ref.png".into()])
            .unwrap();
    let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();
    assert_eq!(
        body["reference_images"][0]["url"],
        "https://example.test/ref.png"
    );
    assert!(body.get("input_reference").is_none());
}

#[test]
fn native_route_preserves_inline_reference_images() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(None, None, vec!["data:image/webp;base64,abc".into()])
        .unwrap();
    let body = CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(
        &req,
        "https://example.test/v1",
    )
    .unwrap();
    assert_eq!(
        body["reference_images"][0]["url"],
        "data:image/webp;base64,abc"
    );
}

#[test]
fn model_names_do_not_block_reference_image_requests_locally() {
    for model in [
        "grok-imagine-video-1.5",
        "grok-imagine-video-1.5-preview",
        "grok-imagine-video-1.5-2026-05-30",
    ] {
        let mut req = request();
        req.model = model.into();
        req.media =
            VideoMediaInput::from_images(None, None, vec!["https://example.test/ref.png".into()])
                .unwrap();

        let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();

        assert_eq!(body["model"], model);
        assert_eq!(
            body["reference_images"][0]["url"],
            "https://example.test/ref.png"
        );
    }
}

#[test]
fn first_frame_and_references_are_rejected() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(
        Some("https://example.test/first.png".into()),
        None,
        vec![
            "https://example.test/ref-1.png".into(),
            "https://example.test/ref-2.png".into(),
        ],
    )
    .unwrap();
    let err = CliProxyOpenAiVideoProvider::request_body(&req).unwrap_err();
    assert!(err.to_string().contains("不允许同时发送"));
}

#[test]
fn reference_limits_are_enforced() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(
        None,
        None,
        (0..8)
            .map(|index| format!("https://example.test/ref-{index}.png"))
            .collect(),
    )
    .unwrap();
    assert!(
        CliProxyOpenAiVideoProvider::request_body(&req)
            .unwrap_err()
            .to_string()
            .contains("最多支持 7 张")
    );

    req.media =
        VideoMediaInput::from_images(None, None, vec!["https://example.test/ref-0.png".into()])
            .unwrap();
    req.duration_secs = 11;
    assert!(
        CliProxyOpenAiVideoProvider::request_body(&req)
            .unwrap_err()
            .to_string()
            .contains("最大时长为 10 秒")
    );
}

#[test]
fn duration_must_match_gateway_range() {
    let mut req = request();
    for duration in [0, 16] {
        req.duration_secs = duration;
        assert!(
            CliProxyOpenAiVideoProvider::request_body(&req)
                .unwrap_err()
                .to_string()
                .contains("1-15 秒")
        );
    }
}

#[test]
fn completed_video_url_supports_native_and_openai_wrapper_payloads() {
    assert_eq!(
        completed_video_url(&json!({
            "request_id": "job-1",
            "status": "done",
            "video": { "url": "https://cdn.example/video.mp4" }
        })),
        Some("https://cdn.example/video.mp4")
    );
    assert_eq!(
        completed_video_url(&json!({
            "id": "job-1",
            "status": "completed",
            "video_url": "https://cdn.example/openai-wrapper.mp4"
        })),
        Some("https://cdn.example/openai-wrapper.mp4")
    );
    assert_eq!(completed_video_url(&json!({ "status": "completed" })), None);
}

#[test]
fn response_parses_all_gateway_id_status_and_url_aliases() {
    for key in ["request_id", "id", "video_id", "provider_job_id"] {
        let mut value = serde_json::Map::new();
        value.insert(key.into(), json!("job-1"));
        assert_eq!(video_id(&Value::Object(value)).unwrap(), "job-1");
    }
    assert_eq!(
        status(&json!({"state": "RUNNING"})).as_deref(),
        Some("running")
    );
    for key in ["download_url", "output_url", "url"] {
        let mut value = serde_json::Map::new();
        value.insert(key.into(), json!("https://cdn.example/video.mp4"));
        assert_eq!(
            completed_video_url(&Value::Object(value)),
            Some("https://cdn.example/video.mp4")
        );
    }
}

#[test]
fn required_status_rejects_missing_status_without_defaulting() {
    let error = required_status(&json!({ "request_id": "job-1" })).expect_err("status is required");
    assert!(error.to_string().contains("响应缺少 status"));
}

#[test]
fn route_detection_and_content_urls_match_base_url() {
    let native = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/v1", "key");
    assert!(!native.uses_openai_route());
    assert_eq!(
        native.content_url("job-1"),
        "http://127.0.0.1:8317/v1/videos/job-1/content?variant=video"
    );

    let wrapper = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/openai/v1", "key");
    assert!(wrapper.uses_openai_route());
    assert_eq!(
        wrapper.content_url("job-1"),
        "http://127.0.0.1:8317/openai/v1/videos/job-1/content?variant=video"
    );
}

#[tokio::test]
async fn generate_posts_to_route_specific_creation_endpoint() {
    for (base_path, expected_path, expected_time_field) in [
        ("/v1", "/v1/videos/generations", "duration"),
        ("/openai/v1", "/openai/v1/videos", "seconds"),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0; 8192];
            let size = stream.read(&mut bytes).await.unwrap();
            let request = String::from_utf8_lossy(&bytes[..size]);
            assert!(request.starts_with(&format!("POST {expected_path} HTTP/1.1")));
            assert!(request.contains(&format!("\"{expected_time_field}\":")));
            let body = r#"{"request_id":"job-1","status":"queued"}"#;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let provider =
            CliProxyOpenAiVideoProvider::new(&format!("http://{addr}{base_path}"), "key");
        let mut req = request();
        req.media = VideoMediaInput::None;
        let result = provider.generate(req, None).await.unwrap();
        assert_eq!(result.provider_job_id.as_deref(), Some("job-1"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn delete_uses_openai_video_endpoint_and_treats_not_found_as_idempotent() {
    for (status, expected) in [
        ("200 OK", VideoJobDeleteOutcome::Deleted),
        ("404 Not Found", VideoJobDeleteOutcome::NotFound),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("delete provider listener");
        let addr = listener.local_addr().expect("delete provider address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("delete request");
            let mut request = vec![0; 4096];
            let size = stream
                .read(&mut request)
                .await
                .expect("read delete request");
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.starts_with("DELETE /openai/v1/videos/job-42 HTTP/1.1"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer key")
            );
            stream
                .write_all(
                    format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .await
                .expect("write delete response");
        });
        let provider = CliProxyOpenAiVideoProvider::new(&format!("http://{addr}/openai/v1"), "key");

        assert_eq!(provider.delete("job-42").await.unwrap(), expected);
        server.await.expect("delete provider task");
    }
}
