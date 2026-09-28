use super::support::*;
use super::*;

#[test]
fn mixed_scene_sources_preserve_separate_portrait_intents() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    let mut newer = package.source_snapshots[0].clone();
    newer.id = 902;
    package.source_snapshots.push(newer);
    let newer_chapters = package
        .source_snapshot_chapters
        .iter()
        .cloned()
        .map(|mut chapter| {
            chapter.id += 10_000;
            chapter.snapshot_id = 902;
            chapter
        })
        .collect::<Vec<_>>();
    package.source_snapshot_chapters.extend(newer_chapters);
    let mut newer_profile = package.source_snapshot_profiles[0].clone();
    newer_profile.id += 10_000;
    newer_profile.snapshot_id = 902;
    package.source_snapshot_profiles.push(newer_profile);
    package
        .projects
        .iter_mut()
        .find(|project| project.id == 2)
        .unwrap()
        .source_snapshot_id = Some(902);
    package.galgame_scenes[0].source_snapshot_chapter_id = 11_001;
    let mut new_plan = package.authoring_documents[0].clone();
    new_plan.id += 10_000;
    new_plan.source_snapshot_id = 902;
    new_plan.inputs_json["sources"][0]["source_snapshot_id"] = json!(902);
    new_plan.inputs_json["sources"][0]["source_snapshot_chapter_id"] = json!(11_001);
    new_plan.document_sha256 = a4n_db::generation::canonical_json_sha256(&(
        &new_plan.inputs_json,
        &new_plan.document_json,
    ))
    .unwrap();
    package.galgame_scenes[0].adaptation_plan_id = Some(new_plan.id);
    package.authoring_documents.push(new_plan);
    let mut old_scene = galgame_scene_record_with_id(2202, 1001, "main", None);
    old_scene.scene_index = 2;
    package.galgame_scenes.push(old_scene);
    package
        .galgame_lines
        .push(galgame_line_record_with_id(2302, 2202));
    for (index, snapshot_id) in [902, 901].into_iter().enumerate() {
        set_galgame_speaker_line(&mut package.galgame_lines[index], 601, "smile", "left");
        let mut spec = generation_spec_for_source_snapshot(snapshot_id);
        spec.id += index as i64;
        spec.project_id = 2;
        spec.source_chapter_id = None;
        spec.source_snapshot_chapter_id = None;
        spec.target_kind = "portrait_expression".into();
        spec.target_entity_kind = "character_profile".into();
        spec.target_entity_id = Some(601);
        spec.params_json =
            galgame_portrait_params(2, 601, "smile", "standing", &[2301 + index as i64]);
        package.generation_specs.push(spec);
    }
    validate_package_manifest(&package)
        .expect("mixed scenes and equal canonical keys in distinct snapshots are valid");
    package.generation_specs[0].params_json["source_line_ids"] = json!([2301, 2302]);
    let error = validate_package_manifest(&package)
        .expect_err("one portrait intent cannot mix scene snapshots");
    assert!(error.contains("目标上下文不一致"), "{error}");
}

#[test]
fn validate_package_manifest_rejects_galgame_cg_params_scene_line_mismatch() {
    let mut package = package_with_galgame_project();
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_cg".to_string();
    spec.target_entity_kind = "galgame_line".to_string();
    spec.target_entity_id = Some(2301);
    spec.params_json = galgame_cg_params(2, 2301, 2202, None);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame CG params scene.id must match the target line scene");

    assert!(err.contains("CG 参数 scene.id 与目标行所属场不一致"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("params_scene_id=2202"));
    assert!(err.contains("actual_scene_id=2201"));
}

#[test]
fn validate_package_manifest_rejects_galgame_cg_missing_note() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_cg".to_string();
    spec.target_entity_kind = "galgame_line".to_string();
    spec.target_entity_id = Some(2301);
    let mut params = galgame_cg_params(2, 2301, 2201, None);
    params
        .as_object_mut()
        .expect("params object")
        .remove("cg_note");
    spec.params_json = params;
    package.generation_specs = vec![spec];

    let err =
        validate_package_manifest(&package).expect_err("Galgame CG params must include cg_note");

    assert!(err.contains("cg_note 必须是非空字符串"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_cg_invalid_importance() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_cg".to_string();
    spec.target_entity_kind = "galgame_line".to_string();
    spec.target_entity_id = Some(2301);
    let mut params = galgame_cg_params(2, 2301, 2201, None);
    params["cg_importance"] = json!("major");
    spec.params_json = params;
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame CG params must use controlled cg_importance");

    assert!(err.contains("cg_importance 无效"));
    assert!(err.contains("major"));
}

#[test]
fn validate_package_manifest_accepts_generation_galgame_portrait_params() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    set_galgame_speaker_line(&mut package.galgame_lines[0], 601, "smile", "left");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = None;
    spec.source_snapshot_chapter_id = None;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[2301]);
    package.generation_specs = vec![spec];

    validate_package_manifest(&package)
        .expect("Galgame portrait params matching target and source snapshot should import");
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_source_chapter_context() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    set_galgame_speaker_line(&mut package.galgame_lines[0], 601, "smile", "left");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[2301]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait specs must use snapshot-level context");

    assert!(err.contains("立绘差分生成意图必须使用源快照级上下文"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_empty_source_line_ids() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait specs must carry source line samples");

    assert!(err.contains("source_line_ids 必须是非空数组"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_source_line_character_mismatch() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    package
        .text_core
        .character_profiles
        .push(character_profile_record(602));
    set_galgame_speaker_line(&mut package.galgame_lines[0], 602, "smile", "left");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[2301]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait source lines must match target profile");

    assert!(err.contains("行角色与目标不一致"));
    assert!(err.contains("speaker_profile_id=Some(602)"));
    assert!(err.contains("expected=601"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_source_line_variant_mismatch() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    set_galgame_speaker_line(&mut package.galgame_lines[0], 601, "sad", "left");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[2301]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait source lines must match expression");

    assert!(err.contains("行 expression 与目标不一致"));
    assert!(err.contains("expected=smile"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_invalid_position_samples() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    set_galgame_speaker_line(&mut package.galgame_lines[0], 601, "smile", "left");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = json!({
        "canonical_key": "galgame:2:portrait:601:expression:smile:pose:standing",
        "asset_kind": "portrait_expression",
        "character_profile_id": 601,
        "expression": "smile",
        "pose": "standing",
        "position_samples": ["left", ""],
        "source_line_ids": [2301]
    });
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait position samples must be strings");

    assert!(err.contains("position_samples 必须是非空字符串数组"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_position_sample_mismatch() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    set_galgame_speaker_line(&mut package.galgame_lines[0], 601, "smile", "right");
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", "standing", &[2301]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait position_samples must match source line positions");

    assert!(err.contains("position_samples 与 source_line_ids 行站位不一致"));
    assert!(err.contains("position_samples"));
    assert!(err.contains("source_positions"));
}

#[test]
fn validate_package_manifest_rejects_galgame_background_params_scene_id_mismatch() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    spec.params_json = galgame_background_params(2, 9999, 1001, 2001);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame background params scene.id must match target_entity_id");

    assert!(err.contains("canonical_key 与目标不一致") || err.contains("scene.id 与目标不一致"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_params_pose_without_expression() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = None;
    spec.source_snapshot_chapter_id = None;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = json!({
        "canonical_key": "galgame:2:portrait:601:pose:standing",
        "asset_kind": "portrait_expression",
        "character_profile_id": 601,
        "pose": "standing",
        "position_samples": ["left"],
        "source_line_ids": [2301]
    });
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame portrait params must require explicit expression");

    assert!(err.contains("expression 必须是非空字符串"));
    assert!(err.contains("generation_spec_id=301"));
}
