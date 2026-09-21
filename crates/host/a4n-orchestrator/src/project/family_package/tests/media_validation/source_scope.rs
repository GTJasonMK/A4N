use super::*;

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
fn validate_media_manifest_rejects_provenance_runtime_identity() {
    let mut package = package_with_derived_project();
    package.media_assets = vec![media_asset_with_kind(MEDIA_KIND_IMAGE)];
    let mut provenance = media_provenance_record(None);
    provenance.provider_job_id = Some("remote-job".to_string());
    package.media_provenance = vec![provenance];

    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("provider runtime identity must fail");
    assert!(err.contains("不能携带 provider_job_id"));

    package.media_provenance = vec![media_provenance_record(Some(json!({
        "nested": {"workflow_run_id": 8}
    })))];
    let err = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("snapshot runtime identity must fail");
    assert!(err.contains("request_snapshot_json 不能携带运行身份 workflow_run_id"));
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
    let media_payload = media_payload_for_assets(&mut package);

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
