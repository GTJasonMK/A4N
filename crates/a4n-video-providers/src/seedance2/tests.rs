use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};

use super::provider::Seedance2VideoProvider;

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
            prompt: "@image_file_1 作为主角参考，跟随 @video_file_1 的镜头节奏，使用 @audio_file_1 的背景音乐".into(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 15,
            fps: 24,
            model: "st-ai/super-seed2".into(),
            media: VideoMediaInput::References {
                images: vec!["https://cdn.example.com/character.png".into()],
                videos: vec!["https://cdn.example.com/camera.mp4".into()],
                audio: vec!["https://cdn.example.com/music.mp3".into()],
            },
        }
}

#[test]
fn builds_omni_reference_request_body() {
    let body = Seedance2VideoProvider::request_body(&request()).expect("valid body");
    assert_eq!(body["model_id"], "st-ai/super-seed2");
    assert_eq!(body["parameters"]["functionMode"], "omni_reference");
    assert_eq!(body["parameters"]["ratio"], "16:9");
    assert_eq!(body["parameters"]["duration"], 15);
    assert_eq!(
        body["parameters"]["image_files"][0],
        "https://cdn.example.com/character.png"
    );
    assert_eq!(
        body["parameters"]["video_files"][0],
        "https://cdn.example.com/camera.mp4"
    );
    assert_eq!(
        body["parameters"]["audio_files"][0],
        "https://cdn.example.com/music.mp3"
    );
    assert!(
        body["parameters"]["prompt"]
            .as_str()
            .unwrap()
            .contains("@image_file_1")
    );
}

#[test]
fn rejects_nonpositive_dimensions_instead_of_using_default_ratio() {
    let mut req = request();
    req.height = 0;

    let err = Seedance2VideoProvider::request_body(&req).expect_err("invalid dimensions");

    assert!(err.to_string().contains("宽度和高度必须为正整数"));
}

#[test]
fn rejects_missing_reference_media() {
    let mut req = request();
    req.media = VideoMediaInput::None;
    let err = Seedance2VideoProvider::request_body(&req).expect_err("missing media");
    assert!(err.to_string().contains("media.type=references"));
}

#[test]
fn rejects_duration_outside_seedance_range_without_clamping() {
    let mut req = request();
    req.duration_secs = 16;

    let err = Seedance2VideoProvider::request_body(&req).expect_err("duration must fail");

    assert!(err.to_string().contains("4-15 秒"));
    assert!(err.to_string().contains("16 秒"));
}

#[test]
fn builds_task_url_from_submit_url() {
    let provider = Seedance2VideoProvider::new("https://api.example.com/task/submit", "key");
    assert_eq!(
        provider.task_url("task-1"),
        "https://api.example.com/task/task-1"
    );
}
