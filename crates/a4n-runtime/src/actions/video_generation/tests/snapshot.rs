use super::*;

#[test]
#[cfg(feature = "embedded-media-providers")]
fn chapter_video_provider_snapshot_uses_image_placeholders() {
    let prompt = prompt_with_metadata();
    let cfg = video_config(GEMINI_VEO_VIDEO_BACKEND);
    let context = ChapterVideoProviderRequestContext {
        prompt,
        cfg,
        request: VideoGenerationRequest {
            prompt: "完整 provider 文本提示词".to_string(),
            negative_prompt: Some("低清晰度".to_string()),
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: Some("data:image/png;base64,REAL_FIRST_FRAME".to_string()),
            last_frame_image_url: Some("data:image/png;base64,REAL_LAST_FRAME".to_string()),
            reference_image_urls: vec!["data:image/png;base64,REAL_REFERENCE_IMAGE".to_string()],
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "boundary_frames".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: BoundaryFrameImageIds {
            first: Some(31),
            last: Some(32),
        },
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: true,
        last_frame_image_url_available: true,
        provider_input_note: None,
    };

    let snapshot = chapter_video_provider_request_snapshot(&context)
        .expect("chapter video snapshot should render");
    let request = snapshot
        .get("request")
        .expect("snapshot should include provider request");
    let serialized = request.to_string();

    assert!(serialized.contains("完整 provider 文本提示词"));
    assert!(serialized.contains("generated_image #31"));
    assert!(serialized.contains("generated_image #32"));
    assert!(serialized.contains("generated_image #11"));
    assert!(!serialized.contains("REAL_FIRST_FRAME"));
    assert!(!serialized.contains("REAL_LAST_FRAME"));
    assert!(!serialized.contains("REAL_REFERENCE_IMAGE"));
}

#[test]
fn linked_video_preview_config_id_uses_positive_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({ "config_id": 9 })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    assert_eq!(
        linked_video_preview_config_id(&prompt, &video)
            .expect("positive config_id should be reused"),
        Some(9)
    );
}

#[test]
fn linked_video_preview_config_id_allows_missing_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_backend": "external-import"
    })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    assert_eq!(
        linked_video_preview_config_id(&prompt, &video)
            .expect("external imports may not have provider config_id"),
        None
    );
}

#[test]
fn linked_video_preview_config_id_rejects_malformed_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({ "config_id": "9" })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    let err = linked_video_preview_config_id(&prompt, &video)
        .expect_err("string config_id must not fall back to default config");

    assert!(err.contains("metadata_json.config_id 必须是正整数"));

    video.metadata_json = Some(json!({ "config_id": 0 }));
    let err = linked_video_preview_config_id(&prompt, &video)
        .expect_err("non-positive config_id must not fall back to default config");

    assert!(err.contains("metadata_json.config_id 必须是正整数"));
}

#[test]
fn reference_media_record_reads_provider_request_snapshot() {
    let request = json!({
        "model_id": "seedance-2-0-pro",
        "parameters": {
            "prompt": "历史快照里的完整参考媒体视频提示词",
            "functionMode": "omni_reference",
            "image_files": ["https://example.test/character.png"],
            "video_files": ["https://example.test/action.mp4"],
            "audio_files": []
        }
    });
    let video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            Some("生成时请求快照".to_string()),
            request
        )
    })));

    let preview = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect("saved snapshot should preview directly");
    let body = preview.steps[0]
        .request
        .as_ref()
        .expect("snapshot preview should expose request");

    assert_eq!(
        body.pointer("/model_id").and_then(Value::as_str),
        Some("seedance-2-0-pro")
    );
    assert_eq!(
        body.pointer("/parameters/image_files/0")
            .and_then(Value::as_str),
        Some("https://example.test/character.png")
    );
    assert_eq!(
        body.pointer("/parameters/video_files/0")
            .and_then(Value::as_str),
        Some("https://example.test/action.mp4")
    );
}

#[test]
fn provider_request_record_rejects_mismatched_workflow_mode() {
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            None,
            json!({
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            })
        )
    })));
    video.workflow_mode = Some("boundary_frames".to_string());

    let err = validate_video_provider_request_snapshot_owner(&video)
        .expect_err("mismatched explicit workflow mode must be rejected");

    assert!(err.contains("workflow_mode=boundary_frames"));
    assert!(err.contains("reference_media_video 类型不匹配"));
}

#[test]
fn provider_request_record_note_reports_unconfirmed_failed_delivery() {
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            Some("生成时请求快照".to_string()),
            json!({
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            })
        )
    })));
    video.status = "failed".to_string();

    let preview = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect("failed video with snapshot should still preview constructed request");
    let note = preview.steps[0]
        .note
        .as_deref()
        .expect("failed record should include delivery note");

    assert!(note.contains("生成时请求快照"));
    assert!(note.contains("无法仅凭该状态确认 provider 是否实际接收请求"));
    assert!(note.contains("构造出的 provider 请求快照"));
}

#[test]
fn provider_request_record_requires_saved_snapshot() {
    let video = reference_media_video_with_metadata(Some(json!({
        "reference_media": {
            "functionMode": "omni_reference",
            "image_files": ["https://example.test/character.png"]
        }
    })));

    let err = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect_err("missing snapshot cannot be previewed");

    assert!(err.contains("缺少 provider 请求快照"));
    assert!(err.contains("无法直接预览"));
}

#[test]
fn provider_request_record_rejects_blank_required_snapshot_fields() {
    let video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": {
            "schema_version": 1,
            "captured_at": "2026-06-29T20:29:00Z",
            "key": "reference_media_video",
            "title": "   ",
            "kind": "video_generation_seedance2_omni_reference",
            "prompt_name": "reference_media_video",
            "state": "ready",
            "request": {
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            }
        }
    })));

    let err = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect_err("blank snapshot title should fail explicitly");

    assert!(err.contains("provider 请求快照字段 title 不能为空"));
}
