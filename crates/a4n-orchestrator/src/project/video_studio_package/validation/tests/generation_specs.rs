use super::{empty_payload, generation_spec_with_target, package, validate_package_manifest};

#[test]
fn validate_video_studio_manifest_rejects_missing_generation_spec_segment_target() {
    let mut package = package();
    package
        .generation_specs
        .push(generation_spec_with_target("video_segment", Some(999)));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("generation spec target must point to package-local segment");

    assert!(err.contains("生成意图引用了不存在的视频片段"));
    assert!(err.contains("target_entity_id=999"));
}

#[test]
fn validate_video_studio_manifest_rejects_unknown_generation_spec_target_kind() {
    let mut package = package();
    package
        .generation_specs
        .push(generation_spec_with_target("legacy_chapter", Some(1)));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("unknown generation spec target kind with id must fail");

    assert!(err.contains("暂不支持生成意图目标类型"));
    assert!(err.contains("legacy_chapter"));
}

#[test]
fn validate_video_studio_manifest_rejects_generation_spec_source_hash_mismatch() {
    let mut package = package();
    let mut spec = generation_spec_with_target("canvas_node", None);
    spec.source_version_hash = "stale".to_string();
    package.generation_specs.push(spec);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("source hash must match source_material");

    assert!(err.contains("source_version_hash 与 source_material 不一致"));
}
