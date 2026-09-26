use super::support::*;
use super::*;

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
fn validate_package_manifest_rejects_generation_spec_runtime_identity() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut spec = generation_spec_record();
    spec.params_json = json!({"nested": {"config_id": 9}});
    package.generation_specs = vec![spec];

    let err = validate_package_manifest(&package).expect_err("runtime identity must fail");

    assert!(err.contains("params_json 不能携带运行身份 config_id"));
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
    package.text_core.chapters[0] = chapter_record(root_chapter_id, 1, None);
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
