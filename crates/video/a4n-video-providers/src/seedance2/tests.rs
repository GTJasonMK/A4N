use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};

use super::Seedance2VideoProvider;

fn request(media: VideoMediaInput) -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "镜头从起始画面平滑移动到结束画面".into(),
        negative_prompt: None,
        width: 1280,
        height: 720,
        duration_secs: 5,
        fps: 24,
        model: "seedance-2-0-pro".into(),
        media,
    }
}

#[test]
fn builds_text_only_request_body() {
    let body =
        Seedance2VideoProvider::request_body(&request(VideoMediaInput::None)).expect("valid body");
    assert_eq!(body["model"], "seedance-2-0-pro");
    assert_eq!(body["seconds"], "5");
    assert_eq!(body["size"], "1280x720");
    assert!(body.get("first_frame_image").is_none());
    assert!(body.get("last_frame_image").is_none());
    assert!(body.get("input_reference").is_none());
}

#[test]
fn negative_constraints_are_part_of_the_submitted_prompt() {
    let mut req = request(VideoMediaInput::None);
    req.negative_prompt = Some("不要改变角色服装".into());
    let body = Seedance2VideoProvider::request_body(&req).unwrap();
    assert!(
        body["prompt"]
            .as_str()
            .unwrap()
            .contains("不要改变角色服装")
    );
    assert!(body.get("negative_prompt").is_none());
}

#[test]
fn builds_first_and_last_frame_request_body() {
    let body = Seedance2VideoProvider::request_body(&request(VideoMediaInput::Frames {
        first: "https://cdn.example.com/first.png".into(),
        last: Some("https://cdn.example.com/last.png".into()),
        references: vec!["https://cdn.example.com/style.png".into()],
    }))
    .expect("valid body");
    assert_eq!(
        body["first_frame_image"],
        "https://cdn.example.com/first.png"
    );
    assert_eq!(body["last_frame_image"], "https://cdn.example.com/last.png");
    assert_eq!(
        body["input_reference"][0],
        "https://cdn.example.com/style.png"
    );
}

#[test]
fn builds_first_frame_only_request_body() {
    let body = Seedance2VideoProvider::request_body(&request(VideoMediaInput::Frames {
        first: "https://cdn.example.com/first.png".into(),
        last: None,
        references: Vec::new(),
    }))
    .expect("valid body");
    assert_eq!(
        body["first_frame_image"],
        "https://cdn.example.com/first.png"
    );
    assert!(body.get("last_frame_image").is_none());
}

#[test]
fn builds_reference_media_request_body() {
    let body = Seedance2VideoProvider::request_body(&request(VideoMediaInput::References {
        images: vec!["https://cdn.example.com/character.png".into()],
        videos: vec!["https://cdn.example.com/camera.mp4".into()],
        audio: vec!["https://cdn.example.com/music.mp3".into()],
    }))
    .expect("valid body");
    assert_eq!(body["input_reference"].as_array().unwrap().len(), 3);
}

#[test]
fn rejects_nonpositive_dimensions_instead_of_using_default_size() {
    let mut req = request(VideoMediaInput::None);
    req.height = 0;

    let err = Seedance2VideoProvider::request_body(&req).expect_err("invalid dimensions");

    assert!(err.to_string().contains("宽度和高度必须为正整数"));
}

#[test]
fn rejects_empty_reference_media() {
    let req = request(VideoMediaInput::References {
        images: Vec::new(),
        videos: Vec::new(),
        audio: Vec::new(),
    });
    let err = Seedance2VideoProvider::request_body(&req).expect_err("missing media");
    assert!(err.to_string().contains("input_reference"));
}

#[test]
fn rejects_duration_outside_seedance_range_without_clamping() {
    let mut req = request(VideoMediaInput::None);
    req.duration_secs = 16;

    let err = Seedance2VideoProvider::request_body(&req).expect_err("duration must fail");

    assert!(err.to_string().contains("4-15 秒"));
    assert!(err.to_string().contains("16 秒"));
}

#[test]
fn builds_video_urls_from_configured_base_url() {
    let provider = Seedance2VideoProvider::new("https://api.example.com/v1", "key");
    assert_eq!(
        provider.generation_url(),
        "https://api.example.com/v1/videos"
    );
    assert_eq!(
        provider.task_url("task-1"),
        "https://api.example.com/v1/videos/task-1"
    );
    assert_eq!(
        provider.content_url("task-1"),
        "https://api.example.com/v1/videos/task-1/content"
    );
}

#[test]
fn extracts_seedance_video_id_aliases() {
    assert_eq!(
        super::response::extract_task_id(&serde_json::json!({"video_id": "video-1"})).as_deref(),
        Some("video-1")
    );
    assert_eq!(
        super::response::extract_task_id(&serde_json::json!({"request_id": "request-1"}))
            .as_deref(),
        Some("request-1")
    );
}
