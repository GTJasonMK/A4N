use a4n_video::{VideoGenerationRequest, VideoMediaInput};
use base64::Engine as _;
use serde_json::json;

use super::provider::HttpVideoProvider;
use super::response::{decode_result, extract_job_id, required_string, required_video_format};

fn provider() -> HttpVideoProvider {
    HttpVideoProvider::new("http://127.0.0.1:9", "test-key")
}

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "animate".into(),
        negative_prompt: Some("noise".into()),
        width: 1280,
        height: 720,
        duration_secs: 5,
        fps: 24,
        model: "video-model".into(),
        media: VideoMediaInput::from_images(
            Some("https://example.test/first.png".into()),
            Some("https://example.test/last.png".into()),
            vec!["https://example.test/ref.png".into()],
        )
        .unwrap(),
    }
}

#[test]
fn request_body_keeps_reference_images_with_boundary_frames() {
    let body = HttpVideoProvider::request_body(&request());
    assert_eq!(body["image"]["url"], "https://example.test/first.png");
    assert_eq!(body["last_frame"]["url"], "https://example.test/last.png");
    assert_eq!(
        body["reference_image_urls"][0],
        "https://example.test/ref.png"
    );
}

#[tokio::test]
async fn decodes_canonical_b64_video_result() {
    let raw = base64::engine::general_purpose::STANDARD.encode(b"video-bytes");
    let value = json!({"data": [{"b64_json": raw, "format": "mp4"}]});
    let result = decode_result(&provider().client, &value, None)
        .await
        .expect("decode succeeds")
        .expect("has result");
    assert_eq!(result.bytes, b"video-bytes");
    assert_eq!(result.format, "mp4");
}

#[tokio::test]
async fn rejects_b64_video_without_format() {
    let raw = base64::engine::general_purpose::STANDARD.encode(b"video-bytes");
    let value = json!({"data": [{"b64_json": raw}]});
    assert!(
        decode_result(&provider().client, &value, None)
            .await
            .is_err()
    );
}

#[test]
fn extracts_only_top_level_id() {
    assert_eq!(
        extract_job_id(&json!({"id": "job-1"})).unwrap().as_deref(),
        Some("job-1")
    );
    assert!(
        extract_job_id(&json!({"job_id": "job-1"}))
            .unwrap()
            .is_none()
    );
    assert!(
        extract_job_id(&json!({"data": {"id": "job-1"}}))
            .unwrap()
            .is_none()
    );
}

#[test]
fn requires_top_level_status() {
    assert_eq!(
        required_string(&json!({"status": "running"}), "/status", "视频任务 status").unwrap(),
        "running"
    );
    assert!(required_string(&json!({"state": "running"}), "/status", "视频任务 status").is_err());
}

#[test]
fn rejects_unknown_video_format() {
    assert!(required_video_format(&json!({"format": "avi"})).is_err());
}
