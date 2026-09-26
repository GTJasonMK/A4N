use super::*;

#[test]
fn validate_package_manifest_rejects_generation_text_anchor_outside_snapshot() {
    let mut package = package_with_derived_project();
    let chapter = chapter_record(201, 1, None);
    let live_anchor = chapter_text_anchor_record(&chapter);
    package.text_core.chapters.push(chapter);
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
