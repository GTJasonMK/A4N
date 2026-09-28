use super::*;

#[test]
fn validate_package_manifest_rejects_galgame_unknown_line_type() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "aside".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("unknown Galgame line types must be rejected");

    assert!(err.contains("Galgame 行 2301 line_type 无效"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_non_positive_index() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_index = 0;

    let err = validate_package_manifest(&package)
        .expect_err("Galgame line_index must match database positive integer contract");

    assert!(err.contains("Galgame 行 line_index 必须是正整数"));
    assert!(err.contains("line_id=2301"));
    assert!(err.contains("line_index=0"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_duplicate_index() {
    let mut package = package_with_galgame_project();
    let mut second_line = package.galgame_lines[0].clone();
    second_line.id = 2302;
    second_line.line_index = 1;
    package.galgame_lines.push(second_line);

    let err = validate_package_manifest(&package)
        .expect_err("Galgame line_index must be unique inside one scene");

    assert!(err.contains("line_index 重复"));
    assert!(err.contains("scene_id=2201"));
    assert!(err.contains("line_index=1"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_index_gap() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_index = 2;

    let err = validate_package_manifest(&package)
        .expect_err("Galgame line_index must remain contiguous inside one scene");

    assert!(err.contains("line_index 必须从 1 连续编号"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_empty_text() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].text = "  ".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("Galgame line text must match database non-empty contract");

    assert!(err.contains("Galgame 行 text 不能为空"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_invalid_position() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].position = Some("foreground".to_string());

    let err = validate_package_manifest(&package)
        .expect_err("unknown Galgame line positions must be rejected");

    assert!(err.contains("Galgame 行 2301 的演出投影列与 presentation_json 不一致"));
}

#[test]
fn validate_package_manifest_rejects_galgame_dialogue_without_speaker() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "dialogue".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("dialogue lines must bind speaker_profile_id");

    assert!(err.contains("必须绑定 speaker_profile_id"));
    assert!(err.contains("Galgame 行 2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_dialogue_projection_mismatch() {
    let mut package = package_with_galgame_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].pose = Some("standing".to_string());
    package.galgame_lines[0].position = Some("center".to_string());
    package.galgame_lines[0].presentation_json =
        speaker_galgame_presentation_json(601, "neutral", "center");

    let err = validate_package_manifest(&package)
        .expect_err("redundant presentation columns must match the canonical document");

    assert!(err.contains("演出投影列与 presentation_json 不一致"));
    assert!(err.contains("Galgame 行 2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_invalid_expression() {
    let mut package = package_with_galgame_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("ecstatic".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    package.galgame_lines[0].position = Some("center".to_string());
    package.galgame_lines[0].presentation_json =
        speaker_galgame_presentation_json(601, "ecstatic", "center");

    let err = validate_package_manifest(&package).expect_err("invalid expression must fail");

    assert!(err.contains("expression 无效"));
    assert!(err.contains("Galgame 行 2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_presentation_json_non_object() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!("cg=true");

    let err = validate_package_manifest(&package)
        .expect_err("Galgame presentation_json must remain an object in import packages");

    assert!(err.contains("presentation_json 不符合 v3 合同"));
    assert!(err.contains("Galgame 行 2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_cg_without_note() {
    let mut package = package_with_galgame_project();
    let mut presentation = empty_galgame_presentation_json();
    presentation["cg"] = json!(true);
    presentation["cg_importance"] = json!("normal");
    package.galgame_lines[0].presentation_json = presentation;

    let err = validate_package_manifest(&package)
        .expect_err("event CG directives in packages must include cg_note");

    assert!(err.contains("cg=true 时 cg_note 不能为空"));
    assert!(err.contains("Galgame 行 2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_invalid_cg_importance() {
    let mut package = package_with_galgame_project();
    let mut presentation = empty_galgame_presentation_json();
    presentation["cg"] = json!(true);
    presentation["cg_note"] = json!("雨中回头");
    presentation["cg_importance"] = json!("major");
    package.galgame_lines[0].presentation_json = presentation;

    let err = validate_package_manifest(&package)
        .expect_err("event CG importance in packages must use the controlled enum");

    assert!(err.contains("presentation_json 不符合 v3 合同"));
    assert!(err.contains("major"));
}

#[test]
fn validate_package_manifest_accepts_galgame_line_null_event_cg_fields() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = empty_galgame_presentation_json();

    validate_package_manifest(&package).expect("disabled event CG fields remain null");
}

#[test]
fn validate_package_manifest_rejects_galgame_line_empty_cg_importance() {
    let mut package = package_with_galgame_project();
    let mut presentation = empty_galgame_presentation_json();
    presentation["cg"] = json!(true);
    presentation["cg_note"] = json!("雨中回头");
    presentation["cg_importance"] = json!(" ");
    package.galgame_lines[0].presentation_json = presentation;

    let err = validate_package_manifest(&package)
        .expect_err("event CG importance must be an explicit canonical enum");

    assert!(err.contains("presentation_json 不符合 v3 合同"));
}

#[test]
fn validate_package_manifest_accepts_galgame_line_event_cg_contract() {
    let mut package = package_with_galgame_project();
    let mut presentation = empty_galgame_presentation_json();
    presentation["cg"] = json!(true);
    presentation["cg_note"] = json!("雨中回头");
    presentation["cg_importance"] = json!("critical");
    package.galgame_lines[0].presentation_json = presentation;

    validate_package_manifest(&package)
        .expect("valid event CG directives should remain importable");
}

#[test]
fn validate_package_manifest_accepts_galgame_line_snapshot_anchor() {
    let mut package = package_with_galgame_project();
    let anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[0]);
    package.galgame_lines[0].text_anchor_id = Some(anchor.id);
    package.text_anchors = vec![anchor];

    validate_package_manifest(&package)
        .expect("Galgame line anchors from the owning scene snapshot chapter should import");
}

#[test]
fn validate_package_manifest_rejects_galgame_line_anchor_from_other_snapshot_chapter() {
    let mut package = package_with_galgame_project();
    let anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[1]);
    package.galgame_lines[0].text_anchor_id = Some(anchor.id);
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame line anchors must belong to the owning scene snapshot chapter");

    assert!(err.contains("text_anchor_id 不属于行所属场源快照章节"));
    assert!(err.contains("line_id=2301"));
    assert!(err.contains("expected_source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_targeting_open_branch() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "branch", None));
    package.galgame_choices = vec![
        galgame_choice_record_with_index(2401, 2301, 1, 2202),
        galgame_choice_record_with_index(2402, 2301, 2, 2202),
    ];

    let err = validate_package_manifest(&package)
        .expect_err("choice targets must not point to unclosed branch scenes");

    assert!(err.contains("Galgame branch 场缺少 next_scene_id"));
    assert!(err.contains("scene_id=2202"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_target_cross_script() {
    let mut package = package_with_galgame_project();
    let mut second_script = package.galgame_scripts[0].clone();
    second_script.id = 2102;
    package.galgame_scripts.push(second_script);
    let mut second_scene = galgame_scene_record_with_id(2202, 1001, "main", None);
    second_scene.script_id = 2102;
    package.galgame_scenes.push(second_scene);
    package.galgame_lines[0].line_type = "choice".to_string();
    package.galgame_choices = vec![
        galgame_choice_record_with_index(2401, 2301, 1, 2202),
        galgame_choice_record_with_index(2402, 2301, 2, 2202),
    ];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame choice target_scene_id must stay inside one script");

    assert!(err.contains("target_scene_id 不属于同一剧本"));
    assert!(err.contains("choice_id=2401"));
}

#[test]
fn validate_package_manifest_accepts_galgame_choice_targeting_closed_branch() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    let mut branch = galgame_scene_record_with_id(2202, 1001, "branch", Some(2203));
    branch.scene_index = 3;
    let mut return_scene = galgame_scene_record_with_id(2203, 1001, "main", None);
    return_scene.scene_index = 2;
    package.galgame_scenes.extend([branch, return_scene]);
    package.galgame_choices = vec![
        galgame_choice_record_with_index(2401, 2301, 1, 2202),
        galgame_choice_record_with_index(2402, 2301, 2, 2202),
    ];

    validate_package_manifest(&package)
        .expect("choice targeting a branch returning to main should import");
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_without_target_scene() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    let mut choice = galgame_choice_record_with_index(2401, 2301, 1, 2201);
    choice.target_scene_id = None;
    package.galgame_choices = vec![
        choice,
        galgame_choice_record_with_index(2402, 2301, 2, 2201),
    ];

    let err = validate_package_manifest(&package)
        .expect_err("galgame choices in a package must have target scenes");

    assert!(err.contains("Galgame 选项缺少 target_scene_id"));
    assert!(err.contains("choice_id=2401"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_line_without_choices() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("choice lines in packages must have options");

    assert!(err.contains("choice 行选项数量必须是 2 到 3 个"));
    assert!(err.contains("line_id=2301"));
    assert!(err.contains("choice_count=0"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_line_with_single_option() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    package.galgame_choices = vec![galgame_choice_record(2401, 2301, 2201)];

    let err = validate_package_manifest(&package)
        .expect_err("choice lines in packages must keep 2-3 options");

    assert!(err.contains("choice 行选项数量必须是 2 到 3 个"));
    assert!(err.contains("line_id=2301"));
    assert!(err.contains("choice_count=1"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_empty_option_text() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    let mut empty_text_choice = galgame_choice_record_with_index(2401, 2301, 1, 2201);
    empty_text_choice.option_text = "  ".to_string();
    package.galgame_choices = vec![
        empty_text_choice,
        galgame_choice_record_with_index(2402, 2301, 2, 2201),
    ];

    let err =
        validate_package_manifest(&package).expect_err("choice option text must not be empty");

    assert!(err.contains("option_text 不能为空"));
    assert!(err.contains("choice_id=2401"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_non_contiguous_option_index() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "choice".to_string();
    package.galgame_choices = vec![
        galgame_choice_record_with_index(2401, 2301, 1, 2201),
        galgame_choice_record_with_index(2402, 2301, 3, 2201),
    ];

    let err =
        validate_package_manifest(&package).expect_err("choice option indexes must be contiguous");

    assert!(err.contains("option_index 必须从 1 连续编号"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_choice_attached_to_non_choice_line() {
    let mut package = package_with_galgame_project();
    package.galgame_choices = vec![galgame_choice_record(2401, 2301, 2201)];

    let err = validate_package_manifest(&package)
        .expect_err("galgame choices must be attached to choice lines");

    assert!(err.contains("Galgame 选项必须挂在 choice 行下"));
    assert!(err.contains("choice_id=2401"));
}
