use super::*;

#[test]
fn validate_video_studio_manifest_rejects_missing_source_external_generated_video() {
    let mut package = package();
    package.media_assets.push(media_asset_without_file());
    package
        .media_provenance
        .push(media_provenance(Some("generated_video:999"), None));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("standard generated video source path must reference package video");

    assert!(err.contains("source_external_path"));
    assert!(err.contains("generated_video_id=999"));
}

#[test]
fn validate_video_studio_manifest_rejects_non_object_request_snapshot() {
    let mut package = package();
    package.media_assets.push(media_asset_without_file());
    package
        .media_provenance
        .push(media_provenance(None, Some(serde_json::json!("bad"))));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("media provenance request snapshot must be object");

    assert!(err.contains("request_snapshot_json 必须是 JSON 对象"));
    assert!(err.contains("media_asset_id=60"));
}

#[test]
fn validate_video_studio_manifest_rejects_missing_request_snapshot_generated_video() {
    let mut package = package();
    package.media_assets.push(media_asset_without_file());
    package.media_provenance.push(media_provenance(
        None,
        Some(serde_json::json!({
            "source": "video_canvas",
            "generated_video_id": 999
        })),
    ));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("request snapshot generated video id must be package-local");

    assert!(err.contains("request_snapshot_json.generated_video_id"));
    assert!(err.contains("id=999"));
}
