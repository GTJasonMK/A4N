use super::*;

#[test]
fn validate_media_manifest_rejects_missing_galgame_binding_target() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING)];
    package.media_bindings = vec![media_binding_record("galgame_scene", "background")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("galgame media binding target must exist in the package");

    assert!(err.contains("媒体绑定目标不存在"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("target_kind=galgame_scene"));
    assert!(err.contains("target_id=201"));
}

#[test]
fn validate_media_manifest_accepts_galgame_line_binding_target() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.media_assets = vec![completed_media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_bindings = vec![media_binding_record("galgame_line", "cg")];
    let media_payload = media_payload_for_assets(&mut package);

    validate_media_manifest(&package, &media_payload)
        .expect("galgame line media binding target present in package should import");
}

#[test]
fn galgame_binding_uses_scene_snapshot_after_partial_regeneration() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.galgame_scripts[0].source_snapshot_id = 902;
    let mut initial_snapshot = source_snapshot_record();
    initial_snapshot.id = 902;
    package.source_snapshots.push(initial_snapshot);
    package.media_assets = vec![completed_media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_bindings = vec![media_binding_record("galgame_line", "cg")];
    let media_payload = media_payload_for_assets(&mut package);
    validate_media_manifest(&package, &media_payload)
        .expect("the target scene owns media source scope");
    package.media_assets[0].source_snapshot_id = Some(902);
    let error = validate_media_manifest(&package, &media_payload)
        .expect_err("script provenance cannot replace scene provenance");
    assert!(error.contains("源快照"), "{error}");
}

#[test]
fn validate_media_manifest_rejects_invalid_galgame_binding_pair() {
    let mut package = package_with_valid_media_binding_target("galgame_scene");
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_bindings = vec![media_binding_record("galgame_scene", "cg")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("galgame scene only accepts background media bindings");

    assert!(err.contains("Galgame 媒体绑定目标/角色无效"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("target_kind=galgame_scene"));
    assert!(err.contains("role=cg"));
}

#[test]
fn validate_media_manifest_rejects_galgame_binding_media_kind_mismatch() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_AUDIO)];
    package.media_bindings = vec![media_binding_record("galgame_line", "cg")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("galgame cg binding must reference an image media asset");

    assert!(err.contains("Galgame 媒体绑定资产类型不匹配"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("required_media_kind=image"));
    assert!(err.contains("actual_media_kind=audio"));
}

#[test]
fn validate_media_manifest_accepts_galgame_audio_binding_media_kind() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.media_assets = vec![completed_media_asset_with_kind(MEDIA_KIND_AUDIO)];
    package.media_bindings = vec![media_binding_record("galgame_line", "bgm")];
    let media_payload = media_payload_for_assets(&mut package);

    validate_media_manifest(&package, &media_payload)
        .expect("galgame bgm binding should accept audio media assets");
}

#[test]
fn validate_media_manifest_rejects_galgame_binding_pending_asset() {
    let mut package = package_with_valid_media_binding_target("galgame_scene");
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_bindings = vec![media_binding_record("galgame_scene", "background")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame media binding must reference a completed asset");

    assert!(err.contains("Galgame 媒体绑定资产不是可用已完成媒体"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_galgame_binding_project_mismatch() {
    let mut package = package_with_valid_media_binding_target("galgame_scene");
    let mut asset = completed_media_asset_with_kind(MEDIA_KIND_IMAGE);
    asset.project_id = 3;
    package.media_assets = vec![asset];
    package.media_bindings = vec![media_binding_record("galgame_scene", "background")];
    let media_payload = media_payload_for_assets(&mut package);

    let err = validate_media_manifest(&package, &media_payload)
        .expect_err("Galgame media binding asset must belong to the target project");

    assert!(err.contains("Galgame 媒体绑定资产与目标项目不一致"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("asset_project_id=3"));
    assert!(err.contains("target_project_id=2"));
}

#[test]
fn validate_media_manifest_rejects_galgame_binding_source_snapshot_mismatch() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    let mut asset = completed_media_asset_with_kind(MEDIA_KIND_IMAGE);
    asset.source_snapshot_id = Some(902);
    package
        .source_snapshots
        .push(project_source_snapshot::Model {
            id: 902,
            root_project_id: 1,
            source_version_hash: "hash-2".to_string(),
            blueprint_version_id: None,
            chapter_scope_json: json!({"start": 1, "end": 10}),
            chapter_hashes_json: json!([]),
            profile_hashes_json: json!([]),
            created_at: dt(),
        });
    package.media_assets = vec![asset];
    package.media_bindings = vec![media_binding_record("galgame_line", "cg")];
    let media_payload = media_payload_for_assets(&mut package);

    let err = validate_media_manifest(&package, &media_payload)
        .expect_err("Galgame media binding source snapshot must match target script");

    assert!(err.contains("Galgame 媒体绑定资产源快照与目标不一致"));
    assert!(err.contains("asset_source_snapshot_id=Some(902)"));
    assert!(err.contains("target_source_snapshot_id=901"));
}

#[test]
fn validate_media_manifest_rejects_galgame_audio_request_missing_target_line() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_AUDIO)];
    package.media_provenance = vec![media_provenance_record(Some(json!({
        "source": "galgame_audio_import",
        "media_kind": "audio",
        "target_line_id": 9999,
        "role": "bgm"
    })))];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame audio import request snapshot target line must exist");

    assert!(err.contains("request_snapshot_json.target_line_id"));
    assert!(err.contains("target_line_id=9999"));
}

#[test]
fn validate_media_manifest_rejects_galgame_audio_request_without_target_line() {
    let mut package = package_with_valid_media_binding_target("galgame_line");
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_AUDIO)];
    package.media_provenance = vec![media_provenance_record(Some(json!({
        "source": "galgame_audio_import",
        "media_kind": "audio",
        "role": "bgm"
    })))];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame audio import request snapshot must include target_line_id");

    assert!(err.contains("request_snapshot_json.target_line_id 缺失"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_galgame_asset_provenance_missing_request_snapshot() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(None);
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame generated media provenance must keep request_snapshot_json");

    assert!(err.contains("Galgame 资产媒体来源缺少 request_snapshot_json"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_galgame_asset_provenance_wrong_source() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(Some(json!({
        "source": "manual_import",
        "target_kind": "galgame_background",
        "target_entity_kind": "galgame_scene",
        "target_entity_id": 2201
    })));
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame generated media provenance must keep galgame_asset_generation source");

    assert!(err.contains("request_snapshot_json.source 必须是 galgame_asset_generation"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_galgame_asset_request_target_mismatch() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_cg".to_string();
    spec.target_entity_kind = "galgame_line".to_string();
    spec.target_entity_id = Some(2301);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(Some(json!({
        "source": "galgame_asset_generation",
        "target_kind": "galgame_cg",
        "target_entity_kind": "galgame_line",
        "target_entity_id": 9999
    })));
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame asset request snapshot target must match generation_spec");

    assert!(err.contains("request_snapshot_json.target_entity_id"));
    assert!(err.contains("target_entity_id=9999"));
}

#[test]
fn validate_media_manifest_rejects_galgame_asset_request_missing_target_entity_id() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(Some(json!({
        "source": "galgame_asset_generation",
        "target_kind": "galgame_background",
        "target_entity_kind": "galgame_scene"
    })));
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame asset request snapshot target_entity_id is required");

    assert!(err.contains("request_snapshot_json.target_entity_id 缺失"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_galgame_asset_request_missing_target_entity_kind() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(Some(json!({
        "source": "galgame_asset_generation",
        "target_kind": "galgame_background",
        "target_entity_id": 2201
    })));
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("Galgame asset request snapshot target_entity_kind is required");

    assert!(err.contains("request_snapshot_json.target_entity_kind 缺失"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_accepts_galgame_asset_request_target_match() {
    let mut package = package_with_galgame_project();
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.project_id = 2;
    spec.root_project_id = 1;
    spec.source_chapter_id = Some(2001);
    spec.source_snapshot_chapter_id = Some(1001);
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(2201);
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(Some(json!({
        "source": "galgame_asset_generation",
        "target_kind": "galgame_background",
        "target_entity_kind": "galgame_scene",
        "target_entity_id": 2201
    })));
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    validate_media_manifest(&package, &empty_media_payload())
        .expect("matching Galgame asset request snapshot target should pass");
}
