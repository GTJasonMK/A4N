use super::*;

#[test]
fn newapi_first_frame_image_accepts_png_and_jpeg_data_urls() {
    ensure_newapi_first_frame_image_supported(Some("data:image/png;base64,abc"))
        .expect("png should be accepted");
    ensure_newapi_first_frame_image_supported(Some("data:image/jpeg;base64,abc"))
        .expect("jpeg should be accepted");
    ensure_newapi_first_frame_image_supported(Some("data:image/jpg;base64,abc"))
        .expect("jpg mime should be accepted");
}

#[test]
fn newapi_first_frame_image_rejects_unsupported_data_url_mime() {
    let err = ensure_newapi_first_frame_image_supported(Some("data:image/webp;base64,abc"))
        .expect_err("webp should be rejected for New API video image");

    assert!(err.contains("New API 视频首帧 image 仅支持 PNG/JPEG"));
    assert!(err.contains("image/webp"));
    assert!(err.contains("不能放进 New API 的首帧 image 字段"));
}

#[test]
fn newapi_first_frame_image_rejects_malformed_data_url() {
    let err = ensure_newapi_first_frame_image_supported(Some("data:image/png,abc"))
        .expect_err("malformed data url should be rejected");

    assert!(err.contains("data URL 格式无效"));
}

#[test]
fn provider_image_transport_matches_backend_media_contract() {
    assert_eq!(
        provider_image_input_transport_with_override(GEMINI_VEO_VIDEO_BACKEND, None).unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(NEWAPI_VIDEO_BACKEND, None).unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(OPENAI_COMPATIBLE_VIDEO_BACKEND, None)
            .unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_default_image_input_transport_with_remote(CLIPROXY_OPENAI_VIDEO_BACKEND, false),
        ProviderImageInputTransport::PublicUrl
    );
    assert_eq!(
        provider_default_image_input_transport_with_remote(CLIPROXY_OPENAI_VIDEO_BACKEND, true),
        ProviderImageInputTransport::RemoteTempUpload
    );
    assert_eq!(
        provider_image_input_transport_with_override(
            OPENAI_COMPATIBLE_VIDEO_BACKEND,
            Some("public_url")
        )
        .unwrap(),
        ProviderImageInputTransport::PublicUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(
            OPENAI_COMPATIBLE_VIDEO_BACKEND,
            Some("remote_temp_upload")
        )
        .unwrap(),
        ProviderImageInputTransport::RemoteTempUpload
    );
    let err = provider_image_input_transport_with_override(
        OPENAI_COMPATIBLE_VIDEO_BACKEND,
        Some("bad-mode"),
    )
    .expect_err("invalid override should be rejected");
    assert!(err.contains("A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效"));
    assert!(err.contains("inline_data_url"));
    assert!(err.contains("public_url"));
    assert!(err.contains("remote_temp_upload"));
    assert!(err.contains("auto"));

    let err =
        provider_image_input_transport_with_override(OPENAI_COMPATIBLE_VIDEO_BACKEND, Some("  "))
            .expect_err("blank configured transport should fail");
    assert!(err.contains("A4N_VIDEO_IMAGE_INPUT_TRANSPORT 不能为空"));

    for legacy_alias in [
        "public-url",
        "url",
        "remote-temp-upload",
        "remote_temp",
        "remote-temp",
        "inline-data-url",
        "inline",
        "data_url",
        "data-url",
    ] {
        let err = provider_image_input_transport_with_override(
            OPENAI_COMPATIBLE_VIDEO_BACKEND,
            Some(legacy_alias),
        )
        .expect_err("legacy transport aliases must be rejected");
        assert!(err.contains("A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效"));
    }

    let err = provider_image_input_transport_with_override(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        Some("inline_data_url"),
    )
    .expect_err("cliproxy video cannot accept inline image payloads");
    assert!(err.contains("cliproxy-openai-video 不支持 base64/data URL"));
}

#[test]
fn provider_generation_workflow_rejects_chat_video_as_unsupported_not_invalid() {
    let mut prompt = prompt_with_metadata();
    prompt.workflow_mode = "chat_video".to_string();

    let err = ensure_provider_generation_workflow(&prompt)
        .expect_err("chat_video is valid but cannot use app provider generation");

    assert!(err.contains("Chat 生视频模式"));
    assert!(err.contains("不使用应用内视频 provider 生成"));
    assert!(!err.contains("workflow_mode 无效"));
}

#[test]
fn prompt_is_first_frame_chain_accepts_only_provider_generation_workflows() {
    let mut prompt = prompt_with_metadata();
    assert!(
        !prompt_is_first_frame_chain(&prompt)
            .expect("boundary frame prompts are provider-generatable")
    );

    prompt.workflow_mode = "first_frame_chain".to_string();
    assert!(
        prompt_is_first_frame_chain(&prompt)
            .expect("first-frame-chain prompts are provider-generatable")
    );

    prompt.workflow_mode = "chat_video".to_string();
    let err = prompt_is_first_frame_chain(&prompt)
        .expect_err("chat_video cannot be mapped to provider frame mode");
    assert!(err.contains("Chat 生视频模式"));
}

#[test]
fn provider_generation_workflow_accepts_storyboard_as_reference_mode_workflow() {
    let mut prompt = prompt_with_metadata();
    prompt.workflow_mode = "storyboard".to_string();

    ensure_provider_generation_workflow(&prompt).expect("storyboard can use provider generation");
    assert_eq!(
        prompt_provider_workflow(&prompt).expect("storyboard should map to provider workflow"),
        ProviderGenerationWorkflow::Storyboard
    );
    assert!(!prompt_is_first_frame_chain(&prompt).expect("storyboard is provider-generatable"));
}

#[test]
fn newapi_rejects_boundary_frame_mode_instead_of_dropping_tail_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop boundary tail frame input");

    assert!(err.contains("只有一个首帧图片输入槽"));
    assert!(err.contains("不能用于首尾帧模式"));
    assert!(err.contains("尾帧图片会被静默丢弃"));
}

#[test]
fn newapi_rejects_storyboard_mode_because_reference_images_are_required() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop the storyboard reference image");

    assert!(err.contains("New API 视频接口不支持 reference_images"));
    assert!(err.contains("不能用于分镜模式"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_reference_mode_allows_boundary_frame_mode_as_reference_images() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect("cliproxy reference mode should send boundary images as ordinary references");

    assert!(note.is_none());
}

#[test]
fn cliproxy_reference_mode_allows_storyboard_mode_as_reference_images() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::Reference,
        1,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect("cliproxy reference mode should send storyboard image as an ordinary reference");

    assert!(note.is_none());
}

#[test]
fn cliproxy_first_frame_mode_rejects_boundary_frame_mode_instead_of_dropping_tail_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::FirstFrame,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy first-frame mode must not silently drop boundary tail frame input");

    assert!(err.contains("CLIProxy / Grok 视频接口"));
    assert!(err.contains("首帧模式"));
    assert!(err.contains("不能用于首尾帧模式"));
    assert!(err.contains("尾帧图片会被静默丢弃"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_first_frame_mode_rejects_storyboard_mode_instead_of_treating_it_as_first_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::FirstFrame,
        1,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy first-frame mode must not treat storyboard image as input_reference");

    assert!(err.contains("CLIProxy / Grok 视频接口"));
    assert!(err.contains("首帧模式"));
    assert!(err.contains("不能用于分镜模式"));
    assert!(note.is_none());
}

#[test]
fn newapi_rejects_material_reference_images_instead_of_dropping_them() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::FirstFrameChain,
        CliproxyGrokReferenceMode::Reference,
        2,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop material reference images");

    assert!(err.contains("不支持提示词参考图输入"));
    assert!(err.contains("绑定了 2 张参考图"));
    assert!(err.contains("已阻止生成以避免参考图被丢弃"));
}

#[test]
fn reference_ids_with_storyboard_image_keeps_storyboard_first_and_deduplicates() {
    let ids = reference_ids_with_storyboard_image(Some(10), &[10, 11, 0, 12, 11]);

    assert_eq!(ids, vec![10, 11, 12]);
}

#[test]
fn newapi_preview_reports_unsupported_image_bindings_without_blocking() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        1,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should report but not block unsupported newapi bindings");

    let note = note.expect("preview note should describe unsupported bindings");
    assert!(note.contains("不能用于首尾帧模式"));
    assert!(note.contains("不支持提示词参考图输入"));
    assert!(note.contains("当前仅展示请求结构"));
}

#[test]
fn cliproxy_rejects_first_frame_plus_material_refs_instead_of_dropping_refs() {
    let mut note = None;
    let err = enforce_cliproxy_video_media_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        "grok-imagine-video",
        CliproxyGrokReferenceMode::Reference,
        2,
        true,
        6,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy must not silently drop refs when input_reference is used");

    assert!(err.contains("不能同时发送 reference_images"));
    assert!(err.contains("绑定了 2 张参考图"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_preview_warns_first_frame_plus_material_refs_without_blocking() {
    let mut note = None;
    enforce_cliproxy_video_media_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        "grok-imagine-video",
        CliproxyGrokReferenceMode::Reference,
        2,
        true,
        6,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should report unsupported cliproxy media combination");

    let note = note.expect("preview note should describe dropped refs");
    assert!(note.contains("不能同时发送 reference_images"));
    assert!(note.contains("当前仅展示请求结构"));
}

#[test]
fn cliproxy_image_to_video_only_models_reject_reference_images_before_provider_call() {
    for model in [
        "grok-imagine-video-1.5",
        "grok-imagine-video-1.5-preview",
        "grok-imagine-video-1.5-2026-05-30",
    ] {
        let mut note = None;
        let err = enforce_cliproxy_video_media_contract(
            CLIPROXY_OPENAI_VIDEO_BACKEND,
            model,
            CliproxyGrokReferenceMode::Reference,
            1,
            false,
            6,
            VideoProviderRequestPurpose::Generate,
            &mut note,
        )
        .expect_err("grok 1.5 models must not send unsupported reference_images");

        assert!(err.contains(model));
        assert!(err.contains("不支持 reference_images"));
        assert!(err.contains("Grok 首帧模式"));
        assert!(note.is_none());
    }
}

#[test]
fn reference_image_ids_rejects_invalid_reference_image_ids_instead_of_dropping_them() {
    let err = media::reference_image_ids(Some(&json!([11, "12"])))
        .expect_err("string ids must not be silently ignored");

    assert!(err.contains("reference_image_ids[1] 必须是正整数"));
}

#[test]
fn reference_image_ids_rejects_duplicate_reference_image_ids() {
    let err = media::reference_image_ids(Some(&json!([11, 11])))
        .expect_err("duplicate ids must not be silently collapsed");

    assert!(err.contains("reference_image_ids 包含重复 ID：11"));
}

#[test]
fn boundary_frame_missing_image_message_requires_both_boundary_images() {
    let mut row = prompt_with_metadata();
    row.shot_number = 3;
    row.metadata_json
        .as_mut()
        .and_then(Value::as_object_mut)
        .expect("prompt metadata should be object")
        .insert(
            "video_unit".to_string(),
            json!({
                "start_frame_number": 3,
                "end_frame_number": 4,
            }),
        );

    let message = boundary_frame_missing_image_message(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("missing tail frame should be reported");

    assert!(message.contains("首尾帧模式第 3"));
    assert!(message.contains("第 4 张尾帧边界图"));
    assert!(message.contains("请先在首尾帧生成里生成对应边界帧图片"));
    assert!(
        boundary_frame_missing_image_message(
            &row,
            &BoundaryFrameImageIds {
                first: Some(31),
                last: Some(32),
            },
        )
        .is_none()
    );
}

#[test]
fn cliproxy_reference_mode_reference_ids_include_boundary_frames_first() {
    let ids = reference_ids_with_boundary_frame_images(
        &BoundaryFrameImageIds {
            first: Some(88),
            last: Some(90),
        },
        &[59, 88, 80],
    );

    assert_eq!(ids, vec![88, 90, 59, 80]);
}
