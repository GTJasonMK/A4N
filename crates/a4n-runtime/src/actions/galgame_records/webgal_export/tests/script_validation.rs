#[test]
fn webgal_package_rejects_unknown_transition() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].presentation_json = json!({"transition": "spin"});

    let err =
        build_webgal_package(temp.path(), &detail, &[]).expect_err("unknown transition fails");

    assert!(err.contains("transition 不支持: spin"));
}

#[test]
fn webgal_package_rejects_invalid_figure_position() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].position = Some("foreground".to_string());

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("invalid figure position must fail instead of skipping changeFigure");

    assert!(err.contains("position 无效"));
    assert!(err.contains("foreground"));
}

#[test]
fn webgal_package_rejects_dialogue_without_speaker_profile() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].speaker_profile_id = None;

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("dialogue without speaker should fail");

    assert!(err.contains("缺少 speaker_profile_id"));
}

#[test]
fn webgal_package_rejects_dialogue_with_missing_speaker_profile() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].speaker_profile_id = Some(999);

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("dialogue with missing speaker profile should fail");

    assert!(err.contains("绑定的角色档案 999 不存在"));
}

#[test]
fn webgal_package_rejects_choice_line_without_choices() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].choices.clear();

    let err = build_webgal_package(temp.path(), &detail, &[]).expect_err("choice without options");

    assert!(err.contains("选项数量必须是 2 到 3 个"));
    assert!(err.contains("choice_count=0"));
}

#[test]
fn webgal_package_rejects_choice_line_with_single_option() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].choices.truncate(1);

    let err = build_webgal_package(temp.path(), &detail, &[]).expect_err("single choice option");

    assert!(err.contains("选项数量必须是 2 到 3 个"));
    assert!(err.contains("choice_count=1"));
}

#[test]
fn webgal_package_rejects_choice_line_with_empty_option_text() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].choices[0].option_text = "  ".to_string();

    let err =
        build_webgal_package(temp.path(), &detail, &[]).expect_err("empty choice option text");

    assert!(err.contains("option_text 不能为空"));
    assert!(err.contains("choice 1"));
}

#[test]
fn webgal_package_rejects_choice_line_with_non_contiguous_option_index() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].choices[1].option_index = 3;

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("non-contiguous choice option indexes");

    assert!(err.contains("option_index 必须从 1 连续编号"));
    assert!(err.contains("expected"));
}

#[test]
fn webgal_package_rejects_choice_attached_to_non_choice_line() {
    let temp = tempdir().expect("temp dir");
    let now = fixed_time();
    let mut detail = webgal_fixture_detail(Vec::new());
    detail
        .scenes
        .get_mut(0)
        .expect("fixture first scene")
        .choices
        .push(choice_model(3, 1001, 3, "错误残留", 200, now));

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("choice attached to a non-choice line");

    assert!(err.contains("挂在非 choice 行下"));
    assert!(err.contains("line_id=1001"));
}

#[test]
fn webgal_package_rejects_empty_script() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes.clear();

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("empty Galgame script cannot be exported");

    assert!(err.contains("没有可导出的场"));
}

#[test]
fn webgal_package_rejects_unknown_line_type() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].line_type = "jump".to_string();

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("unknown Galgame line_type cannot be exported as narration");

    assert!(err.contains("line_type 无效"));
    assert!(err.contains("jump"));
}

#[test]
fn webgal_package_rejects_unknown_scene_entry_kind() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].scene.entry_kind = "side_path".to_string();

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("unknown Galgame scene entry_kind cannot be exported as main");

    assert!(err.contains("entry_kind 无效"));
    assert!(err.contains("side_path"));
}

#[test]
fn webgal_package_rejects_missing_next_scene_target() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].scene.next_scene_id = Some(999);

    let err =
        build_webgal_package(temp.path(), &detail, &[]).expect_err("missing next scene target");

    assert!(err.contains("next_scene_id 指向不存在的场 999"));
}

#[test]
fn webgal_package_rejects_branch_scene_without_return_target() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[1].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();
    detail.scenes[1].scene.next_scene_id = None;

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("branch scene without return target");

    assert!(err.contains("branch 场 200 缺少 next_scene_id"));
}

#[test]
fn webgal_package_rejects_branch_scene_returning_to_branch() {
    let temp = tempdir().expect("temp dir");
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[1].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();
    detail.scenes[1].scene.next_scene_id = Some(300);
    detail.scenes.push(GalgameSceneDetail {
        scene: scene_model(300, 3, "另一支线", None, fixed_time()),
        lines: vec![line_model(
            3000,
            300,
            1,
            "narration",
            None,
            "另一条支线",
            fixed_time(),
        )],
        choices: Vec::<galgame_choice::Model>::new(),
    });
    detail.scenes[2].scene.entry_kind = SCENE_ENTRY_BRANCH.to_string();

    let err = build_webgal_package(temp.path(), &detail, &[])
        .expect_err("branch scene returning to branch");

    assert!(err.contains("next_scene_id 必须指向主线场"));
}
