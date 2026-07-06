use super::*;

#[test]
fn validate_package_entry_path_rejects_traversal() {
    let err = validate_package_entry_path("media/10/../file.mp4").unwrap_err();
    assert!(err.contains("非法"));
}

#[test]
fn validate_media_bytes_rejects_checksum_mismatch() {
    let asset = media_asset_with_checksum(b"expected bytes");
    let err = validate_media_bytes(&asset, b"actual bytes").unwrap_err();
    assert!(err.contains("大小与清单不一致") || err.contains("checksum"));
}

#[test]
fn completed_media_asset_import_uses_pending_insert_status() {
    assert_eq!(
        media_asset_status_for_import_insert(MEDIA_STATUS_COMPLETED),
        MEDIA_STATUS_PENDING
    );
    assert_eq!(
        media_asset_status_for_import_insert(MEDIA_STATUS_RUNNING),
        MEDIA_STATUS_RUNNING
    );
    assert_eq!(
        media_asset_status_for_import_insert(MEDIA_STATUS_FAILED),
        MEDIA_STATUS_FAILED
    );
}

#[test]
fn validate_media_manifest_rejects_unknown_media_status() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_status("bogus")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset status must use known lifecycle states");

    assert!(err.contains("媒体资产状态无效"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("status=bogus"));
}

#[test]
fn validate_media_manifest_accepts_known_non_completed_media_statuses() {
    for status in [
        MEDIA_STATUS_PENDING,
        MEDIA_STATUS_RUNNING,
        MEDIA_STATUS_FAILED,
        MEDIA_STATUS_CANCELLED,
    ] {
        let mut package = package_with_derived_project();
        package.media_assets = vec![media_asset_with_status(status)];

        validate_media_manifest(&package, &empty_media_payload())
            .expect("known non-completed media status should remain importable");
    }
}

#[test]
fn validate_media_manifest_rejects_unknown_media_kind() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_kind("document")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset kind must use known media kinds");

    assert!(err.contains("媒体资产类型无效"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("media_kind=document"));
}

#[test]
fn validate_media_manifest_accepts_known_media_kinds() {
    for kind in [MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO, MEDIA_KIND_AUDIO] {
        let mut package = package_with_derived_project();
        package.media_assets = vec![media_asset_with_kind(kind)];

        validate_media_manifest(&package, &empty_media_payload())
            .expect("known media kind should remain importable");
    }
}

#[test]
fn validate_media_manifest_rejects_unknown_media_origin() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_origin("unknown")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset origin must use known origins");

    assert!(err.contains("媒体资产来源无效"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("origin=unknown"));
}

#[test]
fn validate_media_manifest_accepts_known_media_origins() {
    for origin in [
        MEDIA_ORIGIN_GENERATED,
        MEDIA_ORIGIN_IMPORTED,
        MEDIA_ORIGIN_EXTRACTED_FRAME,
        MEDIA_ORIGIN_ASSEMBLED_PREVIEW,
    ] {
        let mut package = package_with_derived_project();
        package.media_assets = vec![media_asset_with_origin(origin)];

        validate_media_manifest(&package, &empty_media_payload())
            .expect("known media origin should remain importable");
    }
}

#[test]
fn validate_media_manifest_rejects_missing_source_snapshot_id() {
    let mut package = package_with_derived_project();
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.source_snapshot_id = None;
    package.media_assets = vec![asset];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset must keep snapshot context");

    assert!(err.contains("媒体资产缺少 source_snapshot_id"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_unknown_source_snapshot_id() {
    let mut package = package_with_derived_project();
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.source_snapshot_id = Some(999);
    package.media_assets = vec![asset];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset source snapshot must exist in package");

    assert!(err.contains("媒体资产引用了不存在的源文本快照"));
    assert!(err.contains("source_snapshot_id=999"));
}

#[test]
fn validate_media_manifest_rejects_source_snapshot_chapter_mismatch() {
    let mut package = package_with_derived_project();
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.source_snapshot_id = Some(902);
    asset.source_chapter_id = Some(2001);
    asset.source_snapshot_chapter_id = Some(1001);
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

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset chapter context must belong to the same snapshot");

    assert!(err.contains("媒体资产源快照章节不属于目标快照"));
    assert!(err.contains("source_snapshot_id=902"));
    assert!(err.contains("source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_media_manifest_rejects_snapshot_chapter_without_source_chapter() {
    let mut package = package_with_derived_project();
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.source_snapshot_chapter_id = Some(1001);
    package.media_assets = vec![asset];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media asset chapter fields must be paired");

    assert!(err.contains("source_chapter_id 和 source_snapshot_chapter_id 必须同时提供"));
    assert!(err.contains("media_asset_id=10"));
}

#[test]
fn validate_media_manifest_rejects_unknown_media_binding_target_kind() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING)];
    package.media_bindings = vec![media_binding_record("final_assembly", "generated_result")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media binding target_kind must use known target kinds");

    assert!(err.contains("媒体绑定目标类型无效"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("target_kind=final_assembly"));
}

#[test]
fn validate_media_manifest_accepts_known_media_binding_target_kinds() {
    for target_kind in MEDIA_BINDING_TARGET_KINDS {
        let mut package = package_with_valid_media_binding_target(target_kind);
        let (role, media_kind) = match *target_kind {
            "galgame_scene" => ("background", MEDIA_KIND_IMAGE),
            "galgame_line" => ("cg", MEDIA_KIND_IMAGE),
            _ => ("generated_result", MEDIA_KIND_VIDEO),
        };
        package.media_assets = if matches!(*target_kind, "galgame_scene" | "galgame_line") {
            vec![completed_media_asset_with_kind(media_kind)]
        } else {
            vec![media_asset_with_kind(media_kind)]
        };
        package.media_bindings = vec![media_binding_record(target_kind, role)];
        let media_payload = media_payload_for_assets(&package.media_assets);

        validate_media_manifest(&package, &media_payload)
            .expect("known media binding target_kind should remain importable");
    }
}

#[test]
fn validate_media_manifest_rejects_binding_without_media_asset() {
    let mut package = package_with_valid_media_binding_target("galgame_scene");
    package.media_bindings = vec![media_binding_record("galgame_scene", "background")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media binding must reference a packaged media asset");

    assert!(err.contains("媒体绑定引用了不存在的媒体资产"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("media_asset_id=10"));
}

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
    let media_payload = media_payload_for_assets(&package.media_assets);

    validate_media_manifest(&package, &media_payload)
        .expect("galgame line media binding target present in package should import");
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
    let media_payload = media_payload_for_assets(&package.media_assets);

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
    let media_payload = media_payload_for_assets(&package.media_assets);

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
    let media_payload = media_payload_for_assets(&package.media_assets);

    let err = validate_media_manifest(&package, &media_payload)
        .expect_err("Galgame media binding source snapshot must match target script");

    assert!(err.contains("Galgame 媒体绑定资产源快照与目标不一致"));
    assert!(err.contains("asset_source_snapshot_id=Some(902)"));
    assert!(err.contains("target_source_snapshot_id=901"));
}

#[test]
fn validate_media_manifest_rejects_non_object_request_snapshot() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![media_provenance_record(Some(json!("bad")))];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media provenance request_snapshot_json must be an object");

    assert!(err.contains("request_snapshot_json 必须是 JSON 对象"));
    assert!(err.contains("media_asset_id=10"));
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
fn validate_media_manifest_rejects_generation_spec_source_snapshot_mismatch() {
    let mut package = package_with_derived_project();
    push_second_source_snapshot(&mut package, 902);
    let mut spec = generation_spec_for_source_snapshot(902);
    spec.project_id = 2;
    spec.root_project_id = 1;
    package.generation_specs = vec![spec];
    let mut provenance = media_provenance_record(None);
    provenance.generation_spec_id = Some(301);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media provenance generation_spec source snapshot must match asset");

    assert!(err.contains("generation_spec_id 与媒体资产 source_snapshot_id 不一致"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("asset_source_snapshot_id=901"));
    assert!(err.contains("ref_source_snapshot_id=Some(902)"));
}

#[test]
fn validate_media_manifest_rejects_prompt_artifact_source_snapshot_mismatch() {
    let mut package = package_with_derived_project();
    push_second_source_snapshot(&mut package, 902);
    let mut spec = generation_spec_for_source_snapshot(902);
    spec.project_id = 2;
    spec.root_project_id = 1;
    package.generation_specs = vec![spec];
    package.prompt_artifacts = vec![prompt_artifact_record("chat_video_prompt")];
    let mut provenance = media_provenance_record(None);
    provenance.prompt_artifact_id = Some(701);
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("prompt artifact generation spec source snapshot must match asset");

    assert!(
        err.contains("prompt_artifact.generation_spec_id 与媒体资产 source_snapshot_id 不一致")
    );
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("asset_source_snapshot_id=901"));
    assert!(err.contains("ref_source_snapshot_id=Some(902)"));
}

#[test]
fn validate_media_manifest_rejects_source_media_asset_source_snapshot_mismatch() {
    let mut package = package_with_derived_project();
    push_second_source_snapshot(&mut package, 902);
    let mut source_asset = completed_media_asset_with_kind(MEDIA_KIND_IMAGE);
    source_asset.id = 11;
    source_asset.source_snapshot_id = Some(902);
    let mut provenance = media_provenance_record(None);
    provenance.source_media_asset_id = Some(11);
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING), source_asset];
    package.media_provenance = vec![provenance];
    let media_payload = media_payload_for_assets(&package.media_assets);

    let err = validate_media_manifest(&package, &media_payload)
        .expect_err("source media asset source snapshot must match derived asset");

    assert!(err.contains("source_media_asset_id 与媒体资产 source_snapshot_id 不一致"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("asset_source_snapshot_id=901"));
    assert!(err.contains("ref_source_snapshot_id=Some(902)"));
}

#[test]
fn validate_media_manifest_rejects_pending_source_media_asset() {
    let mut package = package_with_derived_project();
    let mut source_asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    source_asset.id = 11;
    let mut provenance = media_provenance_record(None);
    provenance.source_media_asset_id = Some(11);
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING), source_asset];
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("source media asset must be completed and file-backed");

    assert!(err.contains("source_media_asset_id 不是可追溯的已完成媒体"));
    assert!(err.contains("media_asset_id=10"));
    assert!(err.contains("source_media_asset_id=11"));
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

fn push_second_source_snapshot(package: &mut ProjectFamilyPackage, snapshot_id: i64) {
    let mut snapshot = package
        .source_snapshots
        .first()
        .expect("fixture includes source snapshot")
        .clone();
    snapshot.id = snapshot_id;
    snapshot.chapter_hashes_json = json!([]);
    snapshot.profile_hashes_json = json!([]);
    package.source_snapshots.push(snapshot);
}

#[test]
fn validate_media_manifest_rejects_unknown_media_binding_role() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING)];
    package.video_segments = vec![video_segment_record(201)];
    package.media_bindings = vec![media_binding_record("video_segment", "thumbnail")];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("media binding role must use known roles");

    assert!(err.contains("媒体绑定角色无效"));
    assert!(err.contains("media_binding_id=901"));
    assert!(err.contains("role=thumbnail"));
}

#[test]
fn validate_media_manifest_accepts_known_media_binding_roles() {
    for role in MEDIA_BINDING_ROLES {
        let mut package = package_with_valid_media_binding_target("video_segment");
        package.media_assets = vec![media_asset_with_status(MEDIA_STATUS_PENDING)];
        package.media_bindings = vec![media_binding_record("video_segment", role)];

        validate_media_manifest(&package, &empty_media_payload())
            .expect("known media binding role should remain importable");
    }
}
