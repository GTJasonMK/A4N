use super::*;

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
        let mut package = match *target_kind {
            "chapter_video_prompt" => {
                let mut package = crate::project::family_package::tests::video_validation::package_with_video_rows();
                package.chapter_video_prompts[0].reference_image_ids_json = None;
                package.chapter_video_prompts[0].generated_video_id = None;
                package.generated_images.clear();
                package.generated_videos.clear();
                package
            }
            "generated_video" => {
                let mut package = crate::project::family_package::tests::video_validation::package_with_video_rows();
                package.generated_videos[0].file_path = None;
                package.generated_videos[0].source_image_id = None;
                package.generated_videos[0].metadata_json = None;
                package.generated_images.clear();
                package.chapter_video_prompts.clear();
                package
            }
            _ => package_with_valid_media_binding_target(target_kind),
        };
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
        let mut binding = media_binding_record(target_kind, role);
        binding.target_id = match *target_kind {
            "chapter_video_prompt" => 41,
            "generated_video" => 51,
            _ => 201,
        };
        package.media_bindings = vec![binding];
        let media_payload = media_payload_for_assets(&mut package);

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
