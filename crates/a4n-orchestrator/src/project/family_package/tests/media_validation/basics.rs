use super::*;

#[test]
fn validate_package_entry_path_rejects_traversal() {
    let err = validate_package_entry_path("media/10/../file.mp4").unwrap_err();
    assert!(err.contains("不能越界"));
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
