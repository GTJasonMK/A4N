use super::provider::NewApiVideoProvider;
use super::response::{extract_status, extract_task_id, find_video_url};
use a4n_video::VideoGenerationRequest;
use serde_json::json;

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "生成一段视频".into(),
        negative_prompt: Some("低清晰度".into()),
        width: 720,
        height: 1280,
        duration_secs: 5,
        fps: 24,
        model: "grok-imagine-video".into(),
        first_frame_image_url: None,
        last_frame_image_url: None,
        reference_image_urls: Vec::new(),
        reference_media: None,
    }
}

#[test]
fn builds_newapi_request_body() {
    let body = NewApiVideoProvider::request_body(&request()).expect("valid body");

    assert_eq!(body["model"], "grok-imagine-video");
    assert_eq!(body["prompt"], "生成一段视频");
    assert_eq!(body["duration"], 5);
    assert_eq!(body["size"], "720x1280");
    assert!(body.get("image").is_none());
    assert!(body.get("duration_secs").is_none());
    assert!(body.get("reference_image_urls").is_none());
    assert!(body.get("width").is_none());
    assert!(body.get("height").is_none());
    assert!(body.get("fps").is_none());
}

#[test]
fn rejects_material_reference_images_instead_of_dropping_them() {
    let mut req = request();
    req.reference_image_urls
        .push("data:image/png;base64,def".into());

    let err = NewApiVideoProvider::request_body(&req)
        .expect_err("newapi must reject unsupported reference images");

    assert!(err.to_string().contains("不支持素材参考图输入"));
    assert!(err.to_string().contains("当前收到 1 张参考图"));
}

#[test]
fn non_veo_rejects_tail_frame_instead_of_dropping_it() {
    let mut req = request();
    req.first_frame_image_url = Some("data:image/png;base64,start".into());
    req.last_frame_image_url = Some("data:image/png;base64,end".into());

    let err = NewApiVideoProvider::request_body(&req)
        .expect_err("newapi must reject unsupported tail frame");

    assert!(err.to_string().contains("不支持尾帧输入"));
}

#[test]
fn non_veo_uses_first_frame_image() {
    let mut req = request();
    req.first_frame_image_url = Some("data:image/png;base64,start".into());

    let body = NewApiVideoProvider::request_body(&req).expect("valid body");

    assert_eq!(body["image"], "data:image/png;base64,start");
}

#[test]
fn builds_veo_first_frame_request_body() {
    let mut req = request();
    req.model = "veo-3.1-generate-preview".into();
    req.width = 1280;
    req.height = 720;
    req.first_frame_image_url = Some("data:image/png;base64,start".into());

    let body = NewApiVideoProvider::request_body(&req).expect("valid body");

    assert_eq!(body["model"], "veo-3.1-generate-preview");
    assert_eq!(body["duration"], 5);
    assert_eq!(body["duration_seconds"], 5);
    assert_eq!(body["number_of_videos"], 1);
    assert_eq!(body["enhance_prompt"], true);
    assert_eq!(body["size"], "1280x720");
    assert_eq!(body["aspect_ratio"], "16:9");
    assert_eq!(body["metadata"]["durationSeconds"], 5);
    assert_eq!(body["metadata"]["aspectRatio"], "16:9");
    assert_eq!(body["metadata"]["resolution"], "720p");
    assert_eq!(body["metadata"]["negativePrompt"], "低清晰度");
    assert_eq!(body["image"], "data:image/png;base64,start");
    assert!(body.get("last_frame").is_none());
    assert!(body.get("last_frame_image_url").is_none());
    assert!(body.get("reference_images").is_none());
}

#[test]
fn rejects_veo_boundary_frames_instead_of_dropping_tail_frame() {
    let mut req = request();
    req.model = "veo-3.1-generate-preview".into();
    req.first_frame_image_url = Some("data:image/png;base64,start".into());
    req.last_frame_image_url = Some("data:image/png;base64,end".into());

    let err = NewApiVideoProvider::request_body(&req)
        .expect_err("newapi must reject unsupported tail frame");

    assert!(err.to_string().contains("不支持尾帧输入"));
}

#[test]
fn rejects_veo_material_references_when_no_first_frame_is_available() {
    let mut req = request();
    req.model = "veo-3.1-generate-preview".into();
    req.reference_image_urls = vec![
        "data:image/png;base64,asset1".into(),
        "data:image/png;base64,asset2".into(),
    ];

    let err = NewApiVideoProvider::request_body(&req)
        .expect_err("newapi must reject unsupported reference images");

    assert!(err.to_string().contains("不支持素材参考图输入"));
    assert!(err.to_string().contains("当前收到 2 张参考图"));
}

#[test]
fn extracts_task_id_from_top_level_or_data() {
    assert_eq!(
        extract_task_id(&json!({"task_id": "task-1"})).unwrap(),
        Some("task-1".into())
    );
    assert_eq!(
        extract_task_id(&json!({"data": {"id": "task-2"}})).unwrap(),
        Some("task-2".into())
    );
}

#[test]
fn prefers_nested_done_status_over_outer_failure() {
    let value = json!({
        "data": {
            "status": "FAILURE",
            "fail_reason": "upstream returned unrecognized message",
            "data": {
                "status": "done",
                "video": {
                    "url": "https://vidgen.x.ai/video.mp4"
                }
            }
        }
    });

    assert_eq!(extract_status(&value), Some("done"));
    let payload = find_video_url(&value).unwrap().expect("has video url");
    assert_eq!(payload.url, "https://vidgen.x.ai/video.mp4");
    assert_eq!(payload.format, None);
}

#[test]
fn extracts_official_completed_response() {
    let value = json!({
        "task_id": "abcd",
        "status": "completed",
        "url": "https://example.com/result.webm",
        "format": "webm"
    });

    assert_eq!(extract_status(&value), Some("completed"));
    let payload = find_video_url(&value).unwrap().expect("has video url");
    assert_eq!(payload.url, "https://example.com/result.webm");
    assert_eq!(payload.format, Some("webm"));
}
