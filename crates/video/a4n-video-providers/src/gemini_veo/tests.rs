use super::GeminiVeoVideoProvider;
use super::response::{find_video_uri, veo_resolution_from_size};
use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};
use serde_json::json;

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "生成一段视频".into(),
        negative_prompt: Some("低清晰度".into()),
        width: 1280,
        height: 720,
        duration_secs: 8,
        fps: 24,
        model: "veo-3.1-generate-preview".into(),
        media: VideoMediaInput::from_images(
            Some("data:image/png;base64,first".into()),
            Some("data:image/png;base64,last".into()),
            vec!["data:image/jpeg;base64,asset".into()],
        )
        .unwrap(),
    }
}

#[test]
fn builds_native_veo_first_last_frame_body() {
    let body = GeminiVeoVideoProvider::request_body(&request()).expect("valid body");
    let instance = &body["instances"][0];

    assert_eq!(instance["prompt"], "生成一段视频");
    assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
    assert_eq!(instance["image"]["inlineData"]["data"], "first");
    assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
    assert_eq!(instance["lastFrame"]["inlineData"]["data"], "last");
    assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
    assert_eq!(
        instance["referenceImages"][0]["image"]["inlineData"]["mimeType"],
        "image/jpeg"
    );
    assert_eq!(body["parameters"]["sampleCount"], 1);
    assert_eq!(body["parameters"]["durationSeconds"], 8);
    assert_eq!(body["parameters"]["aspectRatio"], "16:9");
    assert_eq!(body["parameters"]["resolution"], "720p");
    assert_eq!(body["parameters"]["negativePrompt"], "低清晰度");
}

#[test]
fn resolution_uses_short_edge_quality_tiers() {
    assert_eq!(veo_resolution_from_size(1920, 1080), "1080p");
    assert_eq!(veo_resolution_from_size(1080, 1080), "1080p");
    assert_eq!(veo_resolution_from_size(3840, 2160), "4k");
    assert_eq!(veo_resolution_from_size(2160, 2160), "4k");
}

#[test]
fn preview_body_redacts_inline_media_payloads() {
    let body = GeminiVeoVideoProvider::request_body_preview(&request()).expect("preview body");
    let instance = &body["instances"][0];

    assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
    assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
    assert!(
        instance["image"]["inlineData"]["data"]
            .as_str()
            .unwrap()
            .contains("已省略内联媒体数据")
    );
    assert_ne!(instance["image"]["inlineData"]["data"], "first");
    assert_ne!(instance["lastFrame"]["inlineData"]["data"], "last");
    assert_ne!(
        instance["referenceImages"][0]["image"]["inlineData"]["data"],
        "asset"
    );
}

#[test]
fn preview_body_accepts_placeholder_media_urls_as_inline_data_shape() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(
        Some("https://example.invalid/a4n-preview/first".into()),
        Some("https://example.invalid/a4n-preview/last".into()),
        vec!["https://example.invalid/a4n-preview/ref".into()],
    )
    .unwrap();

    let body = GeminiVeoVideoProvider::request_body_preview(&req).expect("preview body");
    let instance = &body["instances"][0];

    assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
    assert!(
        instance["image"]["inlineData"]["data"]
            .as_str()
            .unwrap()
            .contains("预览媒体占位")
    );
    assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
}

#[test]
fn rejects_more_than_three_reference_images() {
    let mut req = request();
    req.media = VideoMediaInput::from_images(
        Some("data:image/png;base64,first".into()),
        Some("data:image/png;base64,last".into()),
        vec![
            "data:image/png;base64,a".into(),
            "data:image/png;base64,b".into(),
            "data:image/png;base64,c".into(),
            "data:image/png;base64,d".into(),
        ],
    )
    .unwrap();

    let err = GeminiVeoVideoProvider::request_body(&req).expect_err("too many refs");

    assert!(err.to_string().contains("最多支持 3 张"));
}

#[test]
fn normalizes_base_url_variants() {
    assert_eq!(
        GeminiVeoVideoProvider::new("https://generativelanguage.googleapis.com", "key")
            .api_base_url()
            .unwrap(),
        "https://generativelanguage.googleapis.com/v1beta"
    );
    assert_eq!(
        GeminiVeoVideoProvider::new("https://example.com/v1", "key")
            .api_base_url()
            .unwrap(),
        "https://example.com/v1beta"
    );
    assert_eq!(
        GeminiVeoVideoProvider::new("https://example.com/v1beta", "key")
            .api_base_url()
            .unwrap(),
        "https://example.com/v1beta"
    );
}

#[test]
fn extracts_operation_video_uri_shapes() {
    let value = json!({
        "done": true,
        "response": {
            "generateVideoResponse": {
                "generatedSamples": [{
                    "video": { "uri": "https://example.com/out.mp4" }
                }]
            }
        }
    });

    assert_eq!(find_video_uri(&value), Some("https://example.com/out.mp4"));
}
