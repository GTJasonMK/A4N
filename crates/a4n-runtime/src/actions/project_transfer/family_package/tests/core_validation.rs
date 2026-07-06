use super::*;

#[test]
fn validate_package_manifest_rejects_overlapping_part_ranges() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(102, 2, 10, 20),
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject overlapping part ranges");

    assert!(err.contains("分部章节范围重叠"));
    assert!(err.contains("part_outline_id=101"));
    assert!(err.contains("第1部(1-10)"));
    assert!(err.contains("part_outline_id=102"));
    assert!(err.contains("第2部(10-20)"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_text_core_row_ids() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(101, 2, 11, 20),
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("duplicate old ids would overwrite import id mappings");

    assert!(err.contains("text_core.part_outlines.id"));
    assert!(err.contains("101"));
}

#[test]
fn validate_package_manifest_accepts_adjacent_part_ranges() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(102, 2, 11, 20),
    ]);

    validate_package_manifest(&package).expect("adjacent part ranges should remain importable");
}

#[test]
fn validate_package_manifest_rejects_chapter_outline_reference_for_other_chapter() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapter_outlines = vec![chapter_outline(102, 2)];
    package.text_core.chapters = vec![chapter_record(201, 1, Some(102))];

    let err = validate_package_manifest(&package)
        .expect_err("chapter records must not point at another chapter outline");

    assert!(err.contains("章节大纲属于第2章"));
    assert!(err.contains("chapter_id=201"));
    assert!(err.contains("chapter_number=1"));
    assert!(err.contains("outline_id=102"));
}

#[test]
fn validate_package_manifest_accepts_same_chapter_outline_reference() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapter_outlines = vec![chapter_outline(101, 1)];
    package.text_core.chapters = vec![chapter_record(201, 1, Some(101))];

    validate_package_manifest(&package).expect("same-chapter outline reference should import");
}

#[test]
fn validate_package_manifest_rejects_chapter_outline_part_outside_range() {
    let mut package = package_with_part_outlines(vec![part_outline(101, 1, 1, 10)]);
    package.text_core.chapter_outlines = vec![chapter_outline_with_part(201, 11, 101)];

    let err = validate_package_manifest(&package)
        .expect_err("chapter outline part_id must contain the chapter number");

    assert!(err.contains("章节大纲引用的第1部分部范围不包含该章节"));
    assert!(err.contains("chapter_outline_id=201"));
    assert!(err.contains("chapter_number=11"));
    assert!(err.contains("part_id=101"));
    assert!(err.contains("range=1-10"));
}

#[test]
fn validate_package_manifest_accepts_chapter_outline_part_inside_range() {
    let mut package = package_with_part_outlines(vec![part_outline(101, 1, 1, 10)]);
    package.text_core.chapter_outlines = vec![chapter_outline_with_part(201, 10, 101)];

    validate_package_manifest(&package)
        .expect("chapter outline may reference a part that contains the chapter");
}

#[test]
fn validate_package_manifest_rejects_unknown_chapter_outline_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut outline = chapter_outline(201, 1);
    outline.status = "bogus".to_string();
    package.text_core.chapter_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("chapter outline status must use known states");

    assert!(err.contains("章节大纲状态无效"));
    assert!(err.contains("chapter_outline_id=201"));
    assert!(err.contains("status=bogus"));
}

#[test]
fn validate_package_manifest_accepts_known_chapter_outline_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut outline = chapter_outline(201, 1);
    outline.status = chapter_outline::STATUS_COMPLETED.to_string();
    package.text_core.chapter_outlines = vec![outline];

    validate_package_manifest(&package)
        .expect("completed chapter outline status should remain importable");
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_spec_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut spec = generation_spec_record();
    spec.status = "bogus".to_string();
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation spec status must use known lifecycle states");

    assert!(err.contains("生成意图状态无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("status=bogus"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_spec_statuses() {
    for status in GENERATION_SPEC_STATUSES {
        let mut package = package_with_derived_project();
        let mut spec = generation_spec_for_source_snapshot(901);
        spec.status = (*status).to_string();
        package.generation_specs = vec![spec];

        validate_package_manifest(&package)
            .expect("known generation spec status should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_spec_target_kind() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut spec = generation_spec_record();
    spec.target_kind = "spreadsheet".to_string();
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation spec target_kind must use known target kinds");

    assert!(err.contains("生成意图目标类型无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("target_kind=spreadsheet"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_spec_target_kinds() {
    for target_kind in GENERATION_SPEC_TARGET_KINDS {
        let mut package = package_with_derived_project();
        let mut spec = generation_spec_for_source_snapshot(901);
        spec.target_kind = (*target_kind).to_string();
        package.generation_specs = vec![spec];

        validate_package_manifest(&package)
            .expect("known generation spec target_kind should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_spec_target_entity_kind() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut spec = generation_spec_record();
    spec.target_entity_kind = "timeline_clip".to_string();
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation spec target_entity_kind must use known entity target kinds");

    assert!(err.contains("生成意图目标实体类型无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("target_entity_kind=timeline_clip"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_spec_target_entity_kinds() {
    for target_entity_kind in GENERATION_SPEC_TARGET_ENTITY_KINDS {
        let mut package = package_with_derived_project();
        let mut spec = generation_spec_for_source_snapshot(901);
        spec.target_entity_kind = (*target_entity_kind).to_string();
        package.generation_specs = vec![spec];

        validate_package_manifest(&package)
            .expect("known generation spec target_entity_kind should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_non_object_generation_spec_params() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut spec = generation_spec_record();
    spec.params_json = json!(["not", "an", "object"]);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation spec params_json must keep service object invariant");

    assert!(err.contains("生成意图参数必须是 JSON 对象"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_generation_spec_source_version_hash_mismatch() {
    let mut package = package_with_derived_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.source_version_hash = "stale-source-version".to_string();
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation spec source_version_hash must match its source snapshot");

    assert!(err.contains("生成意图 source_version_hash 与源快照不一致"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("source_snapshot_id=901"));
}

#[test]
fn validate_package_manifest_accepts_generation_refs_matching_source_snapshot() {
    let mut package = package_with_derived_project();
    let root_chapter_id = package.source_snapshot_chapters[0].source_chapter_id;
    package.text_core.chapters = vec![chapter_record(root_chapter_id, 1, None)];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    let mut snapshot_profile = source_snapshot_profile_record("character");
    snapshot_profile.source_profile_id = 601;
    snapshot_profile.payload_json["id"] = json!(601);
    snapshot_profile.payload_hash = hash_json(&snapshot_profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&snapshot_profile)]);
    package.source_snapshot_profiles = vec![snapshot_profile];
    let snapshot_anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[0]);
    package.text_anchors = vec![snapshot_anchor];
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];
    let mut anchor_ref = generation_text_anchor_ref("context");
    anchor_ref.text_anchor_id = 702;
    package.generation_spec_text_anchors = vec![anchor_ref];
    package.generation_spec_entity_refs = vec![generation_entity_ref("character", "subject")];

    validate_package_manifest(&package)
        .expect("generation refs that match the source snapshot should remain importable");
}

#[test]
fn validate_package_manifest_rejects_generation_text_anchor_outside_spec_source_snapshot_chapter() {
    let mut package = package_with_derived_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.source_chapter_id = Some(package.source_snapshot_chapters[0].source_chapter_id);
    spec.source_snapshot_chapter_id = Some(package.source_snapshot_chapters[0].id);
    package.generation_specs = vec![spec];
    package.text_anchors = vec![snapshot_text_anchor_record(
        &package.source_snapshot_chapters[1],
    )];
    let mut anchor_ref = generation_text_anchor_ref("context");
    anchor_ref.text_anchor_id = 702;
    package.generation_spec_text_anchors = vec![anchor_ref];

    let err = validate_package_manifest(&package).expect_err(
        "chapter-scoped generation specs must not reference anchors from another snapshot chapter",
    );

    assert!(err.contains("生成意图文本锚点不属于生成意图源快照章节"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("text_anchor_id=702"));
    assert!(err.contains("expected=1001"));
}

#[test]
fn validate_package_manifest_rejects_generation_spec_partial_source_chapter_pair() {
    let mut package = package_with_derived_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.source_chapter_id = Some(package.source_snapshot_chapters[0].source_chapter_id);
    spec.source_snapshot_chapter_id = None;
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("generation source chapter ids must be provided as a pair");

    assert!(err.contains("source_chapter_id 和 source_snapshot_chapter_id 必须同时提供"));
    assert!(err.contains("generation_spec_id=301"));
}

#[test]
fn validate_package_manifest_rejects_generation_character_profile_target_outside_snapshot() {
    let mut package = package_with_derived_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("character_profile targets must be frozen in the spec source snapshot");

    assert!(err.contains("生成意图目标实体不属于源快照"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("target_entity_kind=character_profile"));
    assert!(err.contains("target_entity_id=601"));
}

#[test]
fn validate_package_manifest_accepts_generation_character_profile_target_in_snapshot() {
    let mut package = package_with_derived_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    let mut snapshot_profile = source_snapshot_profile_record("character");
    snapshot_profile.source_profile_id = 601;
    snapshot_profile.payload_json["id"] = json!(601);
    snapshot_profile.payload_hash = hash_json(&snapshot_profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&snapshot_profile)]);
    package.source_snapshot_profiles = vec![snapshot_profile];
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    package.generation_specs = vec![spec];

    validate_package_manifest(&package)
        .expect("character_profile target present in source snapshot should import");
}

#[test]
fn validate_package_manifest_rejects_generation_galgame_scene_target_missing() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(9999);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("galgame background specs must target a packaged Galgame scene");

    assert!(err.contains("生成意图 Galgame 场目标不存在"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("target_entity_id=9999"));
}

#[test]
fn validate_package_manifest_accepts_generation_galgame_scene_and_line_targets() {
    let mut package = package_with_galgame_project();
    let targets = [
        (
            "galgame_background",
            "galgame_scene",
            2201,
            galgame_background_params(2, 2201, 1001, 2001),
        ),
        (
            "galgame_cg",
            "galgame_line",
            2301,
            galgame_cg_params(2, 2301, 2201, None),
        ),
    ];

    for (target_kind, target_entity_kind, target_entity_id, params_json) in targets {
        let mut spec = generation_spec_for_source_snapshot(901);
        spec.project_id = 2;
        spec.root_project_id = 1;
        spec.source_chapter_id = Some(2001);
        spec.source_snapshot_chapter_id = Some(1001);
        spec.target_kind = target_kind.to_string();
        spec.target_entity_kind = target_entity_kind.to_string();
        spec.target_entity_id = Some(target_entity_id);
        spec.params_json = params_json;
        package.generation_specs = vec![spec];

        validate_package_manifest(&package)
            .expect("Galgame generation target present in package should import");
    }
}

#[test]
fn validate_package_manifest_rejects_generation_galgame_target_project_mismatch() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 3;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    spec.params_json = galgame_background_params(3, 2201, 1001, 2001);
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package)
        .expect_err("Galgame generation spec target must belong to the spec project");

    assert!(err.contains("Galgame 目标上下文不一致"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("target_project_id=2"));
    assert!(err.contains("spec_project_id=3"));
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
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("smile".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    package.galgame_lines[0].position = Some("left".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = None;
    spec.source_snapshot_chapter_id = None;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[2301]);
    package.generation_specs = vec![spec];

    validate_package_manifest(&package)
        .expect("Galgame portrait params matching target and source snapshot should import");
}

#[test]
fn validate_package_manifest_rejects_galgame_portrait_source_chapter_context() {
    let mut package = package_with_galgame_project();
    add_frozen_character_profile(&mut package, 601);
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("smile".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[2301]);
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
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[]);
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
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(602);
    package.galgame_lines[0].expression = Some("smile".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[2301]);
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
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("sad".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[2301]);
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
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("smile".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
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
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("smile".to_string());
    package.galgame_lines[0].pose = Some("standing".to_string());
    package.galgame_lines[0].position = Some("right".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(601);
    spec.params_json = galgame_portrait_params(2, 601, "smile", Some("standing"), &[2301]);
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

#[test]
fn validate_package_manifest_rejects_galgame_unknown_line_type() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "aside".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("unknown Galgame line types must be rejected");

    assert!(err.contains("Galgame 行类型无效"));
    assert!(err.contains("line_id=2301"));
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

    assert!(err.contains("Galgame 行 position 无效"));
    assert!(err.contains("line_id=2301"));
    assert!(err.contains("position=foreground"));
}

#[test]
fn validate_package_manifest_rejects_galgame_dialogue_without_speaker() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].expression = Some("neutral".to_string());

    let err = validate_package_manifest(&package)
        .expect_err("dialogue lines must bind speaker_profile_id");

    assert!(err.contains("必须绑定 speaker_profile_id"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_dialogue_without_expression() {
    let mut package = package_with_galgame_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);

    let err =
        validate_package_manifest(&package).expect_err("dialogue lines must carry expression");

    assert!(err.contains("必须显式提供 expression"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_invalid_expression() {
    let mut package = package_with_galgame_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.galgame_lines[0].line_type = "dialogue".to_string();
    package.galgame_lines[0].speaker_profile_id = Some(601);
    package.galgame_lines[0].expression = Some("ecstatic".to_string());

    let err = validate_package_manifest(&package).expect_err("invalid expression must fail");

    assert!(err.contains("expression 无效"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_presentation_json_non_object() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!("cg=true");

    let err = validate_package_manifest(&package)
        .expect_err("Galgame presentation_json must remain an object in import packages");

    assert!(err.contains("presentation_json 必须是 JSON 对象"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_cg_without_note() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!({
        "cg": true,
        "cg_importance": "normal"
    });

    let err = validate_package_manifest(&package)
        .expect_err("event CG directives in packages must include cg_note");

    assert!(err.contains("启用事件 CG 时必须提供 cg_note"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_rejects_galgame_line_invalid_cg_importance() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!({
        "cg": true,
        "cg_note": "雨中回头",
        "cg_importance": "major"
    });

    let err = validate_package_manifest(&package)
        .expect_err("event CG importance in packages must use the controlled enum");

    assert!(err.contains("cg_importance 无效"));
    assert!(err.contains("line_id=2301"));
}

#[test]
fn validate_package_manifest_accepts_galgame_line_null_event_cg_fields() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!({
        "cg": null,
        "cg_importance": null
    });

    validate_package_manifest(&package)
        .expect("null event CG fields should match the Galgame save-layer defaults");
}

#[test]
fn validate_package_manifest_accepts_galgame_line_empty_cg_importance_default() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!({
        "cg": true,
        "cg_note": "雨中回头",
        "cg_importance": " "
    });

    validate_package_manifest(&package)
        .expect("empty cg_importance should match the Galgame save-layer default");
}

#[test]
fn validate_package_manifest_accepts_galgame_line_event_cg_contract() {
    let mut package = package_with_galgame_project();
    package.galgame_lines[0].presentation_json = json!({
        "cg": true,
        "cg_note": "雨中回头",
        "cg_importance": "critical"
    });

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
    package.galgame_scenes.push(galgame_scene_record_with_id(
        2202,
        1001,
        "branch",
        Some(2201),
    ));
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

#[test]
fn validate_package_manifest_rejects_generation_text_anchor_outside_snapshot() {
    let mut package = package_with_derived_project();
    let chapter = chapter_record(201, 1, None);
    let live_anchor = chapter_text_anchor_record(&chapter);
    package.text_core.chapters = vec![chapter];
    package.text_anchors = vec![live_anchor];
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];
    let mut anchor_ref = generation_text_anchor_ref("context");
    anchor_ref.text_anchor_id = 701;
    package.generation_spec_text_anchors = vec![anchor_ref];

    let err = validate_package_manifest(&package)
        .expect_err("generation text anchors must belong to the spec source snapshot");

    assert!(err.contains("生成意图文本锚点不属于源快照"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("text_anchor_id=701"));
}

#[test]
fn validate_package_manifest_rejects_generation_entity_ref_outside_snapshot() {
    let mut package = package_with_derived_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];
    package.generation_spec_entity_refs = vec![generation_entity_ref("character", "subject")];

    let err = validate_package_manifest(&package)
        .expect_err("generation entity refs must be frozen in the spec source snapshot");

    assert!(err.contains("生成意图实体引用不属于源快照"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("entity_kind=character"));
    assert!(err.contains("entity_id=601"));
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_text_anchor_role() {
    let mut package = package_with_part_outlines(Vec::new());
    package.generation_spec_text_anchors = vec![generation_text_anchor_ref("chapter_hint")];

    let err = validate_package_manifest(&package)
        .expect_err("generation text anchor role must use known roles");

    assert!(err.contains("生成意图文本锚点角色无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("text_anchor_id=501"));
    assert!(err.contains("role=chapter_hint"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_text_anchor_roles() {
    let mut package = package_with_derived_project();
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];
    let mut anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[0]);
    anchor.id = 501;
    package.text_anchors = vec![anchor];

    for role in GENERATION_TEXT_ANCHOR_ROLES {
        package.generation_spec_text_anchors = vec![generation_text_anchor_ref(role)];

        validate_package_manifest(&package)
            .expect("known generation text anchor role should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_entity_kind() {
    let mut package = package_with_part_outlines(Vec::new());
    package.generation_spec_entity_refs = vec![generation_entity_ref("location", "subject")];

    let err = validate_package_manifest(&package)
        .expect_err("generation entity kind must use known entity kinds");

    assert!(err.contains("生成意图实体类型无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("entity_id=601"));
    assert!(err.contains("entity_kind=location"));
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_entity_role() {
    let mut package = package_with_part_outlines(Vec::new());
    package.generation_spec_entity_refs = vec![generation_entity_ref("character", "owner")];

    let err = validate_package_manifest(&package)
        .expect_err("generation entity role must use known roles");

    assert!(err.contains("生成意图实体角色无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("entity_id=601"));
    assert!(err.contains("role=owner"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_entity_roles() {
    let mut package = package_with_derived_project();
    package.text_core.character_profiles = vec![character_profile_record(601)];
    let mut snapshot_profile = source_snapshot_profile_record("character");
    snapshot_profile.source_profile_id = 601;
    snapshot_profile.payload_json["id"] = json!(601);
    snapshot_profile.payload_hash = hash_json(&snapshot_profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&snapshot_profile)]);
    package.source_snapshot_profiles = vec![snapshot_profile];
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];

    for role in GENERATION_ENTITY_ROLES {
        package.generation_spec_entity_refs = vec![generation_entity_ref("character", role)];

        validate_package_manifest(&package)
            .expect("known generation entity role should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_generation_media_ref_role() {
    let mut package = package_with_part_outlines(Vec::new());
    package.generation_spec_media_refs = vec![generation_media_ref("thumbnail")];

    let err = validate_package_manifest(&package)
        .expect_err("generation media ref role must use known roles");

    assert!(err.contains("生成意图媒体角色无效"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("role=thumbnail"));
}

#[test]
fn validate_package_manifest_accepts_known_generation_media_ref_roles() {
    let mut package = package_with_derived_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    package.generation_specs = vec![spec];
    package.media_assets = vec![media_asset_with_checksum(b"expected bytes")];

    for role in GENERATION_MEDIA_REF_ROLES {
        package.generation_spec_media_refs = vec![generation_media_ref(role)];

        validate_package_manifest(&package)
            .expect("known generation media ref role should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_generation_media_ref_to_pending_asset() {
    let mut package = package_with_derived_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    package.generation_specs = vec![spec];
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING)];
    package.generation_spec_media_refs = vec![generation_media_ref("reference")];

    let err = validate_package_manifest(&package)
        .expect_err("generation media ref must point at a completed asset");

    assert!(err.contains("生成意图媒体引用不是可用已完成资产"));
    assert!(err.contains("generation_spec_id=301"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_package_manifest_rejects_unknown_prompt_kind() {
    let mut package = package_with_part_outlines(Vec::new());
    package.prompt_artifacts = vec![prompt_artifact_record("spreadsheet_prompt")];

    let err = validate_package_manifest(&package)
        .expect_err("prompt artifact kind must use renderable prompt kinds");

    assert!(err.contains("提示词类型无效"));
    assert!(err.contains("prompt_artifact_id=701"));
    assert!(err.contains("prompt_kind=spreadsheet_prompt"));
}

#[test]
fn validate_package_manifest_accepts_known_prompt_kinds() {
    let mut package = package_with_derived_project();
    package.generation_specs = vec![generation_spec_for_source_snapshot(901)];

    for prompt_kind in GENERATION_PROMPT_KINDS {
        package.prompt_artifacts = vec![prompt_artifact_record(prompt_kind)];

        validate_package_manifest(&package)
            .expect("known prompt artifact kind should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_project_type() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].project_type = "novel_medium".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject unknown project_type");

    assert!(err.contains("项目类型无效"));
    assert!(err.contains("novel_medium"));
}

#[test]
fn validate_package_manifest_rejects_unknown_project_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].status = "publishing".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject statuses outside the project state machine");

    assert!(err.contains("未知项目状态"));
    assert!(err.contains("publishing"));
}

#[test]
fn validate_package_manifest_rejects_unknown_project_source_sync_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].source_sync_status = "stale".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject unknown source sync states");

    assert!(err.contains("项目源同步状态无效"));
    assert!(err.contains("project_id=1"));
    assert!(err.contains("source_sync_status=stale"));
}

#[test]
fn validate_package_manifest_rejects_root_project_non_current_source_sync_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].source_sync_status = SOURCE_SYNC_ROOT_UPDATED.to_string();

    let err = validate_package_manifest(&package)
        .expect_err("text root projects must not carry derived source sync states");

    assert!(err.contains("文本项目源同步状态必须是 current"));
    assert!(err.contains("project_id=1"));
    assert!(err.contains("source_sync_status=root_updated"));
}

#[test]
fn validate_package_manifest_rejects_root_project_derived_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].derived_project_status = Some(DERIVED_PROJECT_STATUS_DRAFT.to_string());

    let err = validate_package_manifest(&package)
        .expect_err("text root projects must not carry derived-only status");

    assert!(err.contains("文本项目不能包含派生项目状态"));
    assert!(err.contains("project_id=1"));
}

#[test]
fn validate_package_manifest_rejects_derived_project_missing_derived_status() {
    let mut package = package_with_derived_project();
    package.projects[1].derived_project_status = None;

    let err = validate_package_manifest(&package)
        .expect_err("derived projects must carry derived_project_status");

    assert!(err.contains("派生项目缺少 derived_project_status"));
    assert!(err.contains("project_id=2"));
}

#[test]
fn validate_package_manifest_rejects_unknown_derived_project_status() {
    let mut package = package_with_derived_project();
    package.projects[1].derived_project_status = Some("ready".to_string());

    let err = validate_package_manifest(&package)
        .expect_err("derived project status must use known values");

    assert!(err.contains("派生项目状态无效"));
    assert!(err.contains("project_id=2"));
    assert!(err.contains("derived_project_status=ready"));
}

#[test]
fn validate_package_manifest_accepts_known_derived_project_source_sync_statuses() {
    for status in [
        SOURCE_SYNC_CURRENT,
        SOURCE_SYNC_ROOT_UPDATED,
        SOURCE_SYNC_CONFLICT,
        SOURCE_SYNC_DETACHED,
    ] {
        let mut package = package_with_derived_project();
        package.projects[1].source_sync_status = status.to_string();

        validate_package_manifest(&package)
            .expect("known derived project source sync status should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_invalid_project_derivation_scope() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].chapter_scope_json = json!({"start": 0, "end": 10});

    let err = validate_package_manifest(&package)
        .expect_err("derivation chapter scope must match source sync parser constraints");

    assert!(err.contains("派生关系 chapter_scope_json 范围无效"));
    assert!(err.contains("start=0"));
}

#[test]
fn validate_package_manifest_rejects_non_object_project_derivation_options() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].creation_options_json = json!(["not", "an", "object"]);

    let err = validate_package_manifest(&package)
        .expect_err("creation_options_json must keep service object invariant");

    assert!(err.contains("creation_options_json 必须是 JSON 对象"));
    assert!(err.contains("project_derivation_id=911"));
}

#[test]
fn validate_package_manifest_rejects_derivation_scope_mismatched_snapshot_scope() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].chapter_scope_json = json!({"start": 1, "end": 5});

    let err = validate_package_manifest(&package)
        .expect_err("derivation scope and snapshot scope describe the same frozen source");

    assert!(err.contains("派生关系章节范围与源文本快照不一致"));
    assert!(err.contains("project_derivation_id=911"));
    assert!(err.contains("source_snapshot_id=901"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_wrong_root() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].root_project_id = 99;

    let err = validate_package_manifest(&package)
        .expect_err("source snapshots must belong to the package root project");

    assert!(err.contains("源文本快照不属于根项目"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("root_project_id=99"));
}

#[test]
fn validate_package_manifest_rejects_invalid_source_snapshot_scope() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].chapter_scope_json = json!({"start": 10, "end": 1});

    let err = validate_package_manifest(&package)
        .expect_err("source snapshot chapter scope must match service parser constraints");

    assert!(err.contains("源文本快照 chapter_scope_json 范围无效"));
    assert!(err.contains("start=10"));
    assert!(err.contains("end=1"));
}

#[test]
fn validate_package_manifest_rejects_non_array_source_snapshot_hashes() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].profile_hashes_json = json!({});

    let err = validate_package_manifest(&package)
        .expect_err("source snapshot profile hashes must use service array shape");

    assert!(err.contains("profile_hashes_json 必须是数组"));
    assert!(err.contains("source_snapshot_id=901"));
}

#[test]
fn validate_package_manifest_accepts_consistent_source_snapshot_details() {
    let mut package = package_with_derived_project();
    let outline = source_snapshot_outline_record("part");
    let profile = source_snapshot_profile_record("character");
    package.source_snapshot_outlines = vec![outline];
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&profile)]);
    package.source_snapshot_profiles = vec![profile];

    validate_package_manifest(&package)
        .expect("service-shaped source snapshot details must remain importable");
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_missing_detail() {
    let mut package = package_with_derived_project();
    package
        .source_snapshot_chapters
        .retain(|chapter| chapter.chapter_number != 1);

    let err = validate_package_manifest(&package)
        .expect_err("snapshot scope must have one frozen chapter detail per chapter");

    assert!(err.contains("源文本快照缺少章节明细"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("chapter_number=1"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_out_of_scope() {
    let mut package = package_with_derived_project();
    package.source_snapshot_chapters[0].chapter_number = 11;

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter details must stay inside the frozen scope");

    assert!(err.contains("源快照章节不在快照范围内"));
    assert!(err.contains("source_snapshot_chapter_id=1001"));
    assert!(err.contains("chapter_number=11"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_content_hash_mismatch() {
    let mut package = package_with_derived_project();
    package.source_snapshot_chapters[0].content = "篡改后的冻结正文".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter content_hash must describe frozen content");

    assert!(err.contains("content_hash 与正文不一致"));
    assert!(err.contains("source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_hash_row_mismatch() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].chapter_hashes_json[0]["content_hash"] = json!("wrong");

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter_hashes_json must match chapter detail rows");

    assert!(err.contains("chapter_hashes_json 与章节明细 content_hash 不一致"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("chapter_number=1"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_unknown_kind() {
    let mut package = package_with_derived_project();
    package.source_snapshot_outlines = vec![source_snapshot_outline_record("timeline")];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outlines can only freeze service-created outline kinds");

    assert!(err.contains("源快照大纲类型无效"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
    assert!(err.contains("outline_kind=timeline"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_payload_hash_mismatch() {
    let mut package = package_with_derived_project();
    let mut outline = source_snapshot_outline_record("chapter");
    outline.payload_json["title"] = json!("篡改后的大纲");
    package.source_snapshot_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outline payload_hash must describe payload_json");

    assert!(err.contains("源快照大纲 payload_hash 与 payload_json 不一致"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_payload_id_mismatch() {
    let mut package = package_with_derived_project();
    let mut outline = source_snapshot_outline_record("part");
    outline.payload_json["id"] = json!(999);
    outline.payload_hash = hash_json(&outline.payload_json);
    package.source_snapshot_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outline payload identity must match the detail row");

    assert!(err.contains("payload_json.id 与 source_outline_id 不一致"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
    assert!(err.contains("payload_id=999"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_unknown_kind() {
    let mut package = package_with_derived_project();
    package.source_snapshot_profiles = vec![source_snapshot_profile_record("event")];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profiles only freeze persisted profile kinds");

    assert!(err.contains("源快照档案类型无效"));
    assert!(err.contains("source_snapshot_profile_id=1301"));
    assert!(err.contains("profile_kind=event"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_hash_row_mismatch() {
    let mut package = package_with_derived_project();
    let profile = source_snapshot_profile_record("character");
    package.source_snapshot_profiles = vec![profile];
    package.source_snapshots[0].profile_hashes_json =
        json!([{"kind": "character", "id": 401, "hash": "wrong"}]);

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profile_hashes_json must match profile detail rows");

    assert!(err.contains("profile_hashes_json 与档案明细 hash 不一致"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("kind=character"));
    assert!(err.contains("id=401"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_payload_project_mismatch() {
    let mut package = package_with_derived_project();
    let mut profile = source_snapshot_profile_record("character");
    profile.payload_json["project_id"] = json!(99);
    profile.payload_hash = hash_json(&profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&profile)]);
    package.source_snapshot_profiles = vec![profile];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profile payload project_id must match the frozen root");

    assert!(err.contains("payload_json.project_id 与 root_project_id 不一致"));
    assert!(err.contains("source_snapshot_profile_id=1301"));
    assert!(err.contains("payload_project_id=99"));
}

#[test]
fn validate_package_manifest_accepts_consistent_text_anchor_and_profile_evidence() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![profile_evidence_record()];

    validate_package_manifest(&package)
        .expect("service-shaped text anchor and profile evidence should remain importable");
}

#[test]
fn validate_package_manifest_rejects_text_anchor_quote_hash_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let mut anchor = chapter_text_anchor_record(&chapter);
    anchor.quote_hash = "wrong".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package)
        .expect_err("text anchor quote_hash must describe the quote");

    assert!(err.contains("文本锚点 quote_hash 与 quote 不一致"));
    assert!(err.contains("text_anchor_id=701"));
}

#[test]
fn validate_package_manifest_rejects_text_anchor_source_version_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let mut anchor = chapter_text_anchor_record(&chapter);
    anchor.source_version_hash = "stale".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package)
        .expect_err("chapter content anchor source_version_hash must match chapter content");

    assert!(err.contains("文本锚点 source_version_hash 与源内容不一致"));
    assert!(err.contains("text_anchor_id=701"));
}

#[test]
fn validate_package_manifest_rejects_snapshot_text_anchor_source_mismatch() {
    let mut package = package_with_derived_project();
    let mut anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[0]);
    anchor.source_id = package.source_snapshot_chapters[1].id;
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package).expect_err(
        "snapshot content anchor source_id and snapshot chapter id are the same identity",
    );

    assert!(err.contains("source_id 与 source_snapshot_chapter_id 不一致"));
    assert!(err.contains("text_anchor_id=702"));
}

#[test]
fn validate_package_manifest_rejects_unknown_profile_evidence_role() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    let mut evidence = profile_evidence_record();
    evidence.evidence_role = "origin_story".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![evidence];

    let err = validate_package_manifest(&package)
        .expect_err("profile evidence role must use TextAnchorService role set");

    assert!(err.contains("档案证据 evidence_role 无效"));
    assert!(err.contains("origin_story"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_profile_evidence_key() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    let first = profile_evidence_record();
    let mut duplicate = profile_evidence_record();
    duplicate.id = 802;
    duplicate.evidence_role = format!(" {} ", EVIDENCE_ROLE_FIRST_SEEN);
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![first, duplicate];

    let err = validate_package_manifest(&package)
        .expect_err("profile evidence uniqueness uses normalized evidence_role");

    assert!(err.contains("档案证据重复"));
    assert!(err.contains("profile_id=601"));
    assert!(err.contains("text_anchor_id=701"));
    assert!(err.contains(EVIDENCE_ROLE_FIRST_SEEN));
}

fn galgame_background_params(
    project_id: i64,
    scene_id: i64,
    source_snapshot_chapter_id: i64,
    source_chapter_id: i64,
) -> Value {
    json!({
        "canonical_key": format!("galgame:{project_id}:background:scene:{scene_id}"),
        "asset_kind": "background",
        "scene": {
            "id": scene_id,
            "index": 1,
            "title": "开场"
        },
        "source": {
            "source_snapshot_chapter_id": source_snapshot_chapter_id,
            "source_chapter_id": source_chapter_id,
            "chapter_number": 1
        }
    })
}

fn galgame_cg_params(
    project_id: i64,
    line_id: i64,
    scene_id: i64,
    speaker_profile_id: Option<i64>,
) -> Value {
    json!({
        "canonical_key": format!("galgame:{project_id}:cg:line:{line_id}"),
        "asset_kind": "event_cg",
        "line": {
            "id": line_id,
            "line_index": 1,
            "line_type": "dialogue",
            "speaker_profile_id": speaker_profile_id,
            "text": "这里是开场对白。"
        },
        "scene": {
            "id": scene_id,
            "title": "开场"
        },
        "cg_note": "开场特写",
        "cg_importance": "normal"
    })
}

fn galgame_portrait_params(
    project_id: i64,
    profile_id: i64,
    expression: &str,
    pose: Option<&str>,
    source_line_ids: &[i64],
) -> Value {
    let variant_key = match pose {
        Some(pose) => format!("expression:{expression}:pose:{pose}"),
        None => format!("expression:{expression}"),
    };
    json!({
        "canonical_key": format!("galgame:{project_id}:portrait:{profile_id}:{variant_key}"),
        "asset_kind": "portrait_expression",
        "character_profile_id": profile_id,
        "expression": expression,
        "pose": pose,
        "position_samples": ["left"],
        "source_line_ids": source_line_ids
    })
}

fn add_frozen_character_profile(package: &mut ProjectFamilyPackage, profile_id: i64) {
    package.text_core.character_profiles = vec![character_profile_record(profile_id)];
    let mut snapshot_profile = source_snapshot_profile_record("character");
    snapshot_profile.source_profile_id = profile_id;
    snapshot_profile.payload_json["id"] = json!(profile_id);
    snapshot_profile.payload_hash = hash_json(&snapshot_profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&snapshot_profile)]);
    package.source_snapshot_profiles = vec![snapshot_profile];
}
