use super::*;

#[test]
fn validate_package_manifest_rejects_galgame_branch_without_main_return() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].entry_kind = "branch".to_string();
    package.galgame_scenes[0].next_scene_id = None;

    let err = validate_package_manifest(&package)
        .expect_err("branch scenes in a package must return to main scenes");

    assert!(err.contains("Galgame branch 场缺少 next_scene_id"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_branch_returning_to_branch() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].entry_kind = "branch".to_string();
    package.galgame_scenes[0].next_scene_id = Some(2202);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "branch", None));

    let err = validate_package_manifest(&package)
        .expect_err("branch scenes in a package must return to main scenes");

    assert!(err.contains("Galgame branch 场 next_scene_id 必须指向主线场"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_unknown_scene_entry_kind() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].entry_kind = "side_path".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("unknown Galgame scene entry kind must be rejected");

    assert!(err.contains("entry_kind 无效"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_scene_non_positive_index() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].scene_index = 0;

    let err = validate_package_manifest(&package)
        .expect_err("Galgame scene_index must match database positive integer contract");

    assert!(err.contains("Galgame 场 scene_index 必须是正整数"));
    assert!(err.contains("scene_id=2201"));
    assert!(err.contains("scene_index=0"));
}

#[test]
fn validate_package_manifest_rejects_galgame_scene_empty_title() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].title = "  ".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("Galgame scene title must match database non-empty contract");

    assert!(err.contains("Galgame 场 title 不能为空"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_scene_next_self() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].next_scene_id = Some(2201);

    let err = validate_package_manifest(&package)
        .expect_err("Galgame scene next_scene_id must not point to itself");

    assert!(err.contains("next_scene_id 不能指向自身"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_scene_next_cross_script() {
    let mut package = package_with_galgame_project();
    let mut second_script = package.galgame_scripts[0].clone();
    second_script.id = 2102;
    package.galgame_scripts.push(second_script);
    let mut second_scene = galgame_scene_record_with_id(2202, 1001, "main", None);
    second_scene.script_id = 2102;
    package.galgame_scenes.push(second_scene);
    package.galgame_scenes[0].next_scene_id = Some(2202);

    let err = validate_package_manifest(&package)
        .expect_err("Galgame scene next_scene_id must stay inside one script");

    assert!(err.contains("next_scene_id 不属于同一剧本"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_participant_non_integer_profile_id() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].participants_json = json!([
        {
            "character_profile_id": "601",
            "name": "林晓雨"
        }
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("Galgame participants must use integer profile ids");

    assert!(err.contains("participants_json.character_profile_id 必须是正整数"));
    assert!(err.contains("scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_participant_missing_profile() {
    let mut package = package_with_galgame_project();
    package.galgame_scenes[0].participants_json = json!([
        {
            "character_profile_id": 601,
            "name": "林晓雨"
        }
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("Galgame participants must reference packaged root profiles");

    assert!(err.contains("participants_json.character_profile_id 不存在"));
    assert!(err.contains("scene_id=2201"));
    assert!(err.contains("character_profile_id=601"));
}

#[test]
fn validate_package_manifest_accepts_galgame_dialogue_with_expression() {
    let mut package = package_with_galgame_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("neutral".to_string());

    validate_package_manifest(&package)
        .expect("dialogue lines with speaker and expression should import");
}
