#[test]
fn webgal_package_rejects_missing_bound_media_file() {
    let temp = tempdir().expect("temp dir");
    let detail = webgal_fixture_detail(vec![media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/missing.png",
    )]);

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("missing media file should fail");

    assert!(err.contains("WebGAL 媒体文件") || err.contains("无法读取"));
}

#[test]
fn webgal_package_rejects_invalid_media_binding_pair() {
    let temp = tempdir().expect("temp dir");
    write_fixture_file(temp.path(), "projects/10/images/bg.png", b"bg-bytes");
    let detail = webgal_fixture_detail(vec![media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_CG,
        "projects/10/images/bg.png",
    )]);

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("invalid Galgame media binding pair must fail");

    assert!(err.contains("媒体绑定目标/角色无效"));
    assert!(err.contains("target_kind=galgame_scene"));
    assert!(err.contains("role=cg"));
}

#[test]
fn webgal_package_rejects_media_binding_kind_mismatch() {
    let temp = tempdir().expect("temp dir");
    write_fixture_file(
        temp.path(),
        "projects/10/images/not-audio.png",
        b"image-bytes",
    );
    let detail = webgal_fixture_detail(vec![media_summary_with_kind(
        1,
        11,
        GALGAME_ENTITY_LINE,
        1000,
        MEDIA_BINDING_ROLE_BGM,
        MEDIA_KIND_IMAGE,
        "projects/10/images/not-audio.png",
    )]);

    let err =
        build_webgal_package(temp.path(), &detail, &[]).expect_err("media kind mismatch must fail");

    assert!(err.contains("媒体绑定资产类型不匹配"));
    assert!(err.contains("expected_media_kind=audio"));
    assert!(err.contains("actual_media_kind=image"));
}

#[test]
fn webgal_package_rejects_unsupported_audio_extension() {
    let temp = tempdir().expect("temp dir");
    write_fixture_file(temp.path(), "projects/10/audio/bgm.txt", b"audio-bytes");
    let detail = webgal_fixture_detail(vec![media_summary_with_kind(
        1,
        11,
        GALGAME_ENTITY_LINE,
        1000,
        MEDIA_BINDING_ROLE_BGM,
        MEDIA_KIND_AUDIO,
        "projects/10/audio/bgm.txt",
    )]);

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("unsupported WebGAL audio extension must fail");

    assert!(err.contains("WebGAL 音频媒体扩展名不支持"));
    assert!(err.contains("role=bgm"));
    assert!(err.contains("extension=txt"));
}
