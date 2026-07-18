use super::provider::CliProxyOpenAiVideoProvider;
use super::response::*;
use a4n_video::VideoGenerationRequest;
use serde_json::json;

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "animate".into(),
        negative_prompt: Some("noise".into()),
        width: 1280,
        height: 720,
        duration_secs: 5,
        fps: 24,
        model: "grok-imagine-video".into(),
        first_frame_image_url: Some("https://example.test/first.png".into()),
        last_frame_image_url: None,
        reference_image_urls: Vec::new(),
        reference_media: None,
    }
}

#[test]
fn request_body_uses_openai_videos_shape_and_reference_images_by_default() {
    let mut req = request();
    req.first_frame_image_url = None;
    req.reference_image_urls = vec!["https://example.test/ref.png".into()];

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
fn reference_mode_rejects_first_frame_slot_instead_of_promoting_it() {
    let req = request();
    let err = CliProxyOpenAiVideoProvider::request_body(&req).unwrap_err();

    assert!(
        err.to_string()
            .contains("参考模式只读取 reference_image_urls")
    );
}

#[test]
fn first_frame_mode_rejects_tail_frame_instead_of_dropping_it() {
    let mut req = request();
    req.last_frame_image_url = Some("https://example.test/last.png".into());

    let err = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
        &req,
        Some(&json!({ "grok_reference_mode": "first_frame" })),
    )
    .unwrap_err();

    assert!(err.to_string().contains("不支持尾帧输入"));
}

#[test]
fn first_frame_mode_rejects_inline_first_frame() {
    let mut req = request();
    req.first_frame_image_url = Some("data:image/png;base64,abc".into());
    let err = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
        &req,
        Some(&json!({ "grok_reference_mode": "first_frame" })),
    )
    .unwrap_err();
    assert!(err.to_string().contains("http(s)"));
}

#[test]
fn request_body_sends_references_only_without_first_frame() {
    let mut req = request();
    req.first_frame_image_url = None;
    req.reference_image_urls = vec!["https://example.test/ref.png".into()];
    let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();
    assert_eq!(
        body["reference_images"][0]["url"],
        "https://example.test/ref.png"
    );
    assert!(body.get("input_reference").is_none());
}

#[test]
fn image_to_video_only_models_reject_reference_images_locally() {
    for model in [
        "grok-imagine-video-1.5",
        "grok-imagine-video-1.5-preview",
        "grok-imagine-video-1.5-2026-05-30",
    ] {
        let mut req = request();
        req.model = model.into();
        req.first_frame_image_url = None;
        req.reference_image_urls = vec!["https://example.test/ref.png".into()];

        let err = CliProxyOpenAiVideoProvider::request_body(&req).unwrap_err();

        assert!(err.to_string().contains("不支持 reference_images"));
        assert!(err.to_string().contains(model));
    }
}

#[test]
fn first_frame_mode_uses_first_frame_input_reference() {
    let req = request();
    let body = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
        &req,
        Some(&json!({ "grok_reference_mode": "first_frame" })),
    )
    .unwrap();

    assert_eq!(
        body["input_reference"]["image_url"],
        "https://example.test/first.png"
    );
    assert!(body.get("reference_images").is_none());
}

#[test]
fn first_frame_mode_promotes_first_reference_to_input_reference() {
    let mut req = request();
    req.first_frame_image_url = None;
    req.reference_image_urls = vec![
        "https://example.test/ref-1.png".into(),
        "https://example.test/ref-2.png".into(),
    ];

    let body = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
        &req,
        Some(&json!({ "grok_reference_mode": "first_frame" })),
    )
    .unwrap();

    assert_eq!(
        body["input_reference"]["image_url"],
        "https://example.test/ref-1.png"
    );
    assert!(body.get("reference_images").is_none());
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
fn openai_content_route_is_only_used_for_openai_wrapper_base_url() {
    let native = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/v1", "key");
    assert!(!native.supports_openai_content_route());
    assert_eq!(
        native.content_url("job-1"),
        "http://127.0.0.1:8317/v1/videos/job-1/content?variant=video"
    );

    let wrapper = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/openai/v1", "key");
    assert!(wrapper.supports_openai_content_route());
    assert_eq!(
        wrapper.content_url("job-1"),
        "http://127.0.0.1:8317/openai/v1/videos/job-1/content?variant=video"
    );
}
