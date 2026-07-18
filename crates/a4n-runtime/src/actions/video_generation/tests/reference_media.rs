use super::*;

#[test]
fn reference_media_urls_must_use_lowercase_http_scheme() {
    let mut note = None;
    let err = reference_media_urls(
        vec!["HTTPS://example.test/ref.png".to_string()],
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("uppercase scheme must match frontend validation and be rejected");

    assert!(err.contains("小写 http:// 或 https://"));
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn reference_media_preview_note_includes_provider_input_note() {
    let context = ReferenceMediaProviderRequestPreviewContext {
        request: VideoGenerationRequest {
            prompt: "使用 @image_file_1 作为人物服装与脸部一致性参考，镜头从中景缓慢推进，保持场景光线、动作节奏、人物站位和画面质感一致，生成单段参考媒体视频。".to_string(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 7,
            fps: 24,
            model: "seedance-2-0-pro".to_string(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: Some(VideoReferenceMediaRequest {
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
            }),
        },
        config_id: Some(9),
        provider_backend: Some("newapi-video".to_string()),
        provider_input_note: Some(
            "当前配置不是 seedance2-video；预览仍展示请求结构，真实生成会失败".to_string(),
        ),
    };

    let preview = reference_media_provider_request_preview(
        "reference_media_video_generation",
        "参考媒体视频模型请求",
        "当前表单",
        &context,
    )
    .expect("reference media preview should render");

    let note = preview.steps[0]
        .note
        .as_deref()
        .expect("preview note should be present");
    assert!(note.contains("backend=newapi-video"));
    assert!(note.contains("当前配置不是 seedance2-video"));
    assert!(note.contains("真实生成会失败"));
}

#[test]
fn reference_media_preview_helpers_use_placeholders_for_incomplete_inputs() {
    let mut note = None;
    let mut prompt = reference_media_prompt("", VideoProviderRequestPurpose::Preview, &mut note)
        .expect("preview prompt should use placeholder");
    let urls = reference_media_urls(
        vec!["ftp://example.test/ref.png".to_string()],
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview urls should use placeholders");
    ensure_reference_media_tokens(
        &mut prompt,
        "image_file",
        urls.len(),
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should append missing token placeholders");

    assert_eq!(
        urls,
        vec!["https://example.invalid/a4n-preview/image_files/1".to_string()]
    );
    assert!(prompt.contains(REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER));
    assert!(prompt.contains("@image_file_1"));
    let note = note.expect("preview helper should report incomplete inputs");
    assert!(note.contains("prompt 过短"));
    assert!(note.contains("未以小写 http:// 或 https:// 开头"));
    assert!(note.contains("缺少 @image_file_1"));
}

#[test]
fn reference_media_prompt_must_reference_every_bound_token() {
    let mut prompt = "这是一段足够长的参考媒体视频提示词，用于描述人物外观、场景结构、镜头调度、动作节奏、光影方向、情绪变化和画面风格；其中明确使用 @image_file_1 作为主角造型参考，但故意遗漏第二张图片引用以验证绑定校验。".to_string();
    let mut note = None;

    let err = ensure_reference_media_tokens(
        &mut prompt,
        "image_file",
        2,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("missing token must be rejected");

    assert!(err.contains("@image_file_2"));
}

#[test]
fn reference_media_prompt_requires_substantial_direct_prompt() {
    let mut note = None;
    let err = reference_media_prompt(
        "@image_file_1 做参考",
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("short prompt must be rejected");

    assert!(err.contains("prompt 过短"));
}

#[test]
fn reference_media_aspect_ratio_maps_to_seedance_dimensions() {
    assert_eq!(
        reference_media_dimensions_for_aspect_ratio("21:9").expect("valid ratio"),
        (1680, 720)
    );
    assert_eq!(
        reference_media_dimensions_for_aspect_ratio("3:4").expect("valid ratio"),
        (768, 1024)
    );
    assert!(reference_media_dimensions_for_aspect_ratio("2:1").is_err());
}

#[test]
fn video_mime_from_path_rejects_unknown_video_extensions() {
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mp4")).expect("mp4 MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.M4V")).expect("m4v MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.webm")).expect("webm MIME"),
        "video/webm"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mov")).expect("mov MIME"),
        "video/quicktime"
    );

    let err = media::video_mime_from_path(Path::new("clip.avi"))
        .expect_err("unsupported extension should fail");
    assert!(err.contains("视频文件扩展名不支持播放：avi"));

    let err =
        media::video_mime_from_path(Path::new("clip")).expect_err("missing extension should fail");
    assert_eq!(err, "视频文件缺少扩展名，无法确定 MIME 类型");
}
