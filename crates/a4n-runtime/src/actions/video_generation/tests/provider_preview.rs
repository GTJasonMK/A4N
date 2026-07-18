use super::*;

#[test]
#[cfg(feature = "embedded-media-providers")]
fn cliproxy_preview_rejects_invalid_grok_reference_mode() {
    let mut context = provider_preview_context(CLIPROXY_OPENAI_VIDEO_BACKEND);
    context.cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "invalid-mode"
    }));

    let err = video_provider_preview_payload(&context)
        .expect_err("invalid grok_reference_mode must not silently fall back");

    assert!(err.contains("grok_reference_mode 无效"));
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn cliproxy_reference_preview_sends_reference_images_without_input_reference() {
    let mut context = provider_preview_context(CLIPROXY_OPENAI_VIDEO_BACKEND);
    context.request.first_frame_image_url = None;
    context.request.last_frame_image_url = None;
    context.request.reference_image_urls = vec![
        "https://example.invalid/a4n-preview/ref-1".into(),
        "https://example.invalid/a4n-preview/ref-2".into(),
    ];
    context.prompt_reference_image_ids = Some(json!([11, 12]));
    context.provider_reference_image_ids = Some(json!([11, 12]));
    context.prompt_reference_image_count = 2;
    context.provider_reference_image_count = 2;
    context.first_frame_image_url_available = false;
    context.last_frame_image_url_available = false;

    let (_kind, _title, note, request) =
        video_provider_preview_payload(&context).expect("cliproxy reference preview should render");

    assert!(note.contains("所有输入图片均作为 reference_images 发送"));
    assert!(note.contains("不会发送 input_reference"));
    assert_eq!(
        request["reference_images"][0]["url"],
        "https://example.invalid/a4n-preview/ref-1"
    );
    assert_eq!(
        request["reference_images"][1]["url"],
        "https://example.invalid/a4n-preview/ref-2"
    );
    assert!(request.get("input_reference").is_none());
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn provider_preview_reports_missing_first_frame_chain_start_without_hiding_request() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [], "coverage_check": "无参考图"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "无素材引用", "revision": "无需修订"}
        },
        "start_first_frame": null
    }));
    let frame_ids = BoundaryFrameImageIds {
        first: None,
        last: None,
    };
    let provider_prompt = build_video_generation_provider_prompt(&row, &frame_ids)
        .expect("preview should still build the text prompt");
    let context = ChapterVideoProviderRequestContext {
        prompt: row,
        cfg: video_config(NEWAPI_VIDEO_BACKEND),
        request: VideoGenerationRequest {
            prompt: provider_prompt,
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "first_frame_chain".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: frame_ids,
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: false,
        last_frame_image_url_available: false,
        provider_input_note: Some(
            "当前首帧串联起始帧缺失：首帧串联第 2 个视频单元缺少上一视频单元尾帧图片；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                .to_string(),
        ),
    };

    let (_, _, note, request) =
        video_provider_preview_payload(&context).expect("preview payload should render");

    assert!(note.contains("当前首帧串联起始帧缺失"));
    assert!(note.contains("真实生成仍需补齐该输入"));
    assert!(request.get("image").is_none());
    assert!(request.get("prompt").is_some());
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn provider_preview_uses_openai_compatible_request_body() {
    let context = provider_preview_context(OPENAI_COMPATIBLE_VIDEO_BACKEND);

    let (kind, title, note, request) =
        video_provider_preview_payload(&context).expect("preview payload");

    assert_eq!(kind, "video_generation_openai_compatible");
    assert!(title.contains("/videos/generations"));
    assert!(note.contains("OpenAI 兼容视频"));
    assert!(note.contains("实际发送 prompt："));
    assert!(note.contains("UTF-8 字节"));
    assert_eq!(request["model"], "test-video-model");
    assert_eq!(
        request["image"]["url"],
        "https://example.invalid/a4n-preview/first"
    );
    assert_eq!(
        request["last_frame"]["url"],
        "https://example.invalid/a4n-preview/last"
    );
    assert_eq!(request["response_format"], "b64_json");
    assert!(request.get("reference_media").is_none());
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn provider_preview_uses_gemini_veo_request_body_shape() {
    let context = provider_preview_context(GEMINI_VEO_VIDEO_BACKEND);

    let (kind, title, note, request) =
        video_provider_preview_payload(&context).expect("preview payload");

    assert_eq!(kind, "video_generation_gemini_veo");
    assert!(title.contains("predictLongRunning"));
    assert!(note.contains("inlineData 占位"));
    let instance = &request["instances"][0];
    assert_eq!(instance["prompt"], "导演提示词正文");
    assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
    assert!(
        instance["image"]["inlineData"]["data"]
            .as_str()
            .unwrap()
            .contains("预览媒体占位")
    );
    assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
    assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
    assert_eq!(request["parameters"]["durationSeconds"], 4);
    assert!(request.get("first_frame_image_url").is_none());
    assert!(request.get("reference_image_urls").is_none());
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn provider_preview_rejects_unknown_backend_without_internal_request_fallback() {
    let context = provider_preview_context("unknown-video");

    let err = video_provider_preview_payload(&context)
        .expect_err("unknown backend must not expose internal request fallback");

    assert!(err.contains("未知视频 backend=unknown-video"));
    assert!(err.contains("无法展示真实 provider 请求体"));
}

#[test]
fn provider_preview_duration_note_reports_unsupported_duration_without_blocking() {
    let mut cfg = video_config(NEWAPI_VIDEO_BACKEND);
    cfg.duration_mode = "fixed".to_string();
    cfg.min_duration_secs = 4;
    cfg.max_duration_secs = 8;
    cfg.supported_durations_json = Some(json!([4, 8]));
    let mut note = None;

    append_preview_duration_note(&mut note, &cfg, 7);

    let note = note.expect("unsupported preview duration should be reported");
    assert!(note.contains("不支持 7 秒"));
    assert!(note.contains("预览仍展示请求结构"));
    assert!(note.contains("真实生成会失败"));
}
