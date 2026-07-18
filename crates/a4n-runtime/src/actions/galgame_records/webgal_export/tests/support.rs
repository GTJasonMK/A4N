fn webgal_fixture_detail(media_bindings: Vec<GalgameMediaBindingSummary>) -> GalgameScriptDetail {
    let now = fixed_time();
    GalgameScriptDetail {
        script: galgame_script::Model {
            id: 1,
            project_id: 10,
            root_project_id: 1,
            source_snapshot_id: 20,
            title: "测试 Galgame".to_string(),
            style_config_json: json!({}),
            status: "ready".to_string(),
            created_at: now,
            updated_at: now,
        },
        speaker_profiles: vec![GalgameSpeakerProfile {
            id: 1,
            name: "林晓雨".to_string(),
        }],
        text_anchors: Vec::<GalgameTextAnchorSummary>::new(),
        media_bindings,
        scenes: vec![
            GalgameSceneDetail {
                scene: scene_model(100, 1, "第一场", Some(200), now),
                lines: vec![
                    line_model(1000, 100, 1, "dialogue", Some(1), "你好;世界\n下一行", now),
                    line_model(1001, 100, 2, "narration", None, "雨停了", now),
                    line_model(1002, 100, 3, "choice", None, "选择", now),
                ],
                choices: vec![
                    choice_model(1, 1002, 1, "继续:回想", 200, now),
                    choice_model(2, 1002, 2, "留在|原地", 200, now),
                ],
            },
            GalgameSceneDetail {
                scene: scene_model(200, 2, "第二场", None, now),
                lines: vec![line_model(
                    2000,
                    200,
                    1,
                    "narration",
                    None,
                    "第二场开始",
                    now,
                )],
                choices: Vec::<galgame_choice::Model>::new(),
            },
        ],
    }
}

fn scene_model(
    id: i64,
    scene_index: i32,
    title: &str,
    next_scene_id: Option<i64>,
    now: NaiveDateTime,
) -> galgame_scene::Model {
    galgame_scene::Model {
        id,
        script_id: 1,
        source_snapshot_chapter_id: id + 1000,
        scene_index,
        title: title.to_string(),
        location: None,
        time_of_day: None,
        participants_json: json!([]),
        background_note: None,
        entry_kind: "main".to_string(),
        next_scene_id,
        created_at: now,
        updated_at: now,
    }
}

fn line_model(
    id: i64,
    scene_id: i64,
    line_index: i32,
    line_type: &str,
    speaker_profile_id: Option<i64>,
    text: &str,
    now: NaiveDateTime,
) -> galgame_line::Model {
    galgame_line::Model {
        id,
        scene_id,
        line_index,
        line_type: line_type.to_string(),
        speaker_profile_id,
        text: text.to_string(),
        expression: None,
        pose: None,
        position: None,
        presentation_json: json!({}),
        text_anchor_id: None,
        created_at: now,
        updated_at: now,
    }
}

fn choice_model(
    id: i64,
    line_id: i64,
    option_index: i32,
    option_text: &str,
    target_scene_id: i64,
    now: NaiveDateTime,
) -> galgame_choice::Model {
    galgame_choice::Model {
        id,
        line_id,
        option_index,
        option_text: option_text.to_string(),
        target_scene_id: Some(target_scene_id),
        created_at: now,
    }
}

fn media_summary(
    binding_id: i64,
    asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    file_path: &str,
) -> GalgameMediaBindingSummary {
    let media_kind = if role == MEDIA_BINDING_ROLE_BGM || role == MEDIA_BINDING_ROLE_SFX {
        MEDIA_KIND_AUDIO
    } else {
        MEDIA_KIND_IMAGE
    };
    media_summary_with_kind(
        binding_id,
        asset_id,
        target_kind,
        target_id,
        role,
        media_kind,
        file_path,
    )
}

fn media_summary_with_kind(
    binding_id: i64,
    asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    media_kind: &str,
    file_path: &str,
) -> GalgameMediaBindingSummary {
    let now = fixed_time();
    GalgameMediaBindingSummary {
        binding: media_binding::Model {
            id: binding_id,
            media_asset_id: asset_id,
            target_kind: target_kind.to_string(),
            target_id,
            role: role.to_string(),
            created_at: now,
        },
        asset: media_asset::Model {
            id: asset_id,
            project_id: 10,
            root_project_id: 1,
            source_snapshot_id: Some(20),
            source_chapter_id: Some(30),
            source_snapshot_chapter_id: Some(40),
            media_kind: media_kind.to_string(),
            origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            file_path: Some(file_path.to_string()),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_secs: None,
            fps: None,
            file_size: None,
            checksum: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            created_at: now,
            updated_at: now,
        },
    }
}

fn portrait_model(
    id: i64,
    profile_id: i64,
    expression: Option<&str>,
    pose: Option<&str>,
    is_primary: bool,
    image_path: &str,
) -> character_portrait::Model {
    character_portrait::Model {
        id,
        profile_id,
        project_id: 1,
        portrait_type: "full_body".to_string(),
        image_path: image_path.to_string(),
        prompt: "portrait".to_string(),
        expression: expression.map(str::to_string),
        pose: pose.map(str::to_string),
        is_primary,
        created_at: fixed_time(),
    }
}

fn write_fixture_file(data_dir: &Path, relative_path: &str, bytes: &[u8]) {
    let path = data_dir.join(relative_path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
    std::fs::write(path, bytes).expect("write fixture");
}

fn assert_zip_text_contains(zip: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str, needles: &[&str]) {
    let mut text = String::new();
    zip.by_name(path)
        .unwrap_or_else(|err| panic!("missing zip text entry {path}: {err}"))
        .read_to_string(&mut text)
        .unwrap_or_else(|err| panic!("read zip text entry {path}: {err}"));
    for needle in needles {
        assert!(
            text.contains(needle),
            "zip entry {path} did not contain {needle:?}; content:\n{text}"
        );
    }
}

fn assert_zip_bytes(zip: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str, expected: &[u8]) {
    let mut bytes = Vec::new();
    zip.by_name(path)
        .unwrap_or_else(|err| panic!("missing zip binary entry {path}: {err}"))
        .read_to_end(&mut bytes)
        .unwrap_or_else(|err| panic!("read zip binary entry {path}: {err}"));
    assert_eq!(bytes, expected, "zip entry bytes differ: {path}");
}

fn fixed_time() -> NaiveDateTime {
    chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("valid timestamp")
        .naive_utc()
}
