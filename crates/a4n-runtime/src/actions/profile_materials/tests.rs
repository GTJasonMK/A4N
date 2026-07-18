use super::contracts::{
    IMAGE_TYPE_OBJECT, IMAGE_TYPE_SCENE, KIND_OBJECT, KIND_SCENE, ProfileMaterialImageSummary,
    ProfileMaterialProfileSummary, ProfileMaterialSource, UploadProfileMaterialImageInput,
};
use super::list::{apply_group_filters, apply_group_sort, material_group, material_state};
use super::metadata::{ProfileMaterialStyleMetadata, profile_material_metadata};
use super::queries::material_identity;
use super::upload::{
    append_cleanup_error, decode_uploaded_profile_material_image,
    persist_uploaded_profile_material_image,
};
use crate::actions::image_generation::to_summary;
use a4n_core::models::ai::generated_image;
use chrono::NaiveDate;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn material_identity_requires_current_profile_metadata() {
    let scene = image(
        1,
        IMAGE_TYPE_SCENE,
        json!({ "profile_kind": KIND_SCENE, "profile_id": 42 }),
    );
    let object = image(
        2,
        IMAGE_TYPE_OBJECT,
        json!({ "profile_kind": KIND_OBJECT, "profile_id": 7 }),
    );
    let old_metadata = image(3, IMAGE_TYPE_SCENE, json!({ "scene_profile_id": "42" }));

    assert_eq!(material_identity(&scene), Some((KIND_SCENE, 42)));
    assert_eq!(material_identity(&object), Some((KIND_OBJECT, 7)));
    assert_eq!(material_identity(&old_metadata), None);
}

#[test]
fn profile_material_metadata_uses_current_profile_fields() {
    let metadata = profile_material_metadata(
        KIND_SCENE,
        9,
        "东市",
        "hash",
        &ProfileMaterialStyleMetadata {
            style_template_id: Some("kyoani-everyday-anime".to_string()),
            style_template_name: Some("京阿尼动画风格".to_string()),
            style_template_version: Some("1".to_string()),
            custom_style: None,
        },
    );

    assert!(metadata.get("scene_profile_id").is_none());
    assert_eq!(metadata["profile_kind"], json!("scene"));
    assert_eq!(metadata["profile_id"], json!(9));
    assert_eq!(metadata["profile_content_sha256"], json!("hash"));
    assert_eq!(
        metadata["style_template_id"],
        json!("kyoani-everyday-anime")
    );
}

#[test]
fn material_state_prefers_missing_and_stale_over_ready() {
    assert_eq!(material_state(&[]), "missing");

    let fresh = image_summary(1, "fresh");
    assert_eq!(material_state(&[fresh]), "ready");

    let stale = image_summary(2, "stale");
    assert_eq!(material_state(&[image_summary(1, "fresh"), stale]), "stale");

    let unknown = image_summary(3, "unknown");
    assert_eq!(material_state(&[unknown]), "unknown");
}

#[test]
fn favorite_filter_uses_all_profile_images_not_only_visible_limit() {
    let mut images_by_profile = BTreeMap::from([(
        (KIND_SCENE.to_string(), 1),
        vec![
            image_at(
                1,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                1,
                false,
            ),
            image_at(
                2,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                2,
                true,
            ),
        ],
    )]);
    let primary_by_profile = BTreeMap::from([((KIND_SCENE.to_string(), 1), 1)]);
    let group = material_group(
        material_source(1, "档案大厅"),
        &mut images_by_profile,
        &primary_by_profile,
        1,
    );
    assert_eq!(group.images.len(), 1);
    assert!(!group.images[0].image.is_favorite);

    let mut groups = vec![group];
    apply_group_filters(&mut groups, None, Some("favorite"));

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].profile.id, 1);
}

#[test]
fn newest_sort_uses_all_profile_images_not_only_visible_limit() {
    let mut first_images = BTreeMap::from([(
        (KIND_SCENE.to_string(), 1),
        vec![
            image_at(
                1,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                1,
                false,
            ),
            image_at(
                10,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                3,
                false,
            ),
        ],
    )]);
    let first_primary = BTreeMap::from([((KIND_SCENE.to_string(), 1), 1)]);
    let first = material_group(
        material_source(1, "档案大厅"),
        &mut first_images,
        &first_primary,
        1,
    );
    assert_eq!(first.images[0].image.id, 1);

    let mut second_images = BTreeMap::from([(
        (KIND_SCENE.to_string(), 2),
        vec![image_at(
            5,
            IMAGE_TYPE_SCENE,
            json!({ "profile_kind": KIND_SCENE, "profile_id": 2 }),
            2,
            false,
        )],
    )]);
    let second = material_group(
        material_source(2, "旧书房"),
        &mut second_images,
        &BTreeMap::new(),
        1,
    );

    let mut groups = vec![second, first];
    apply_group_sort(&mut groups, Some("newest"));

    assert_eq!(groups[0].profile.id, 1);
    assert_eq!(groups[1].profile.id, 2);
}

#[test]
fn decode_uploaded_profile_material_image_accepts_png_base64_and_data_url() {
    for data in [
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    ] {
        let input = upload_input("image/png", data);
        let decoded = decode_uploaded_profile_material_image(&input).expect("valid png upload");

        assert_eq!(decoded.mime_type, "image/png");
        assert_eq!(decoded.extension, "png");
        assert_eq!((decoded.width, decoded.height), (1, 1));
    }
}

#[test]
fn decode_uploaded_profile_material_image_rejects_unsupported_mime() {
    let input = upload_input(
        "image/gif",
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    );

    let err =
        decode_uploaded_profile_material_image(&input).expect_err("unsupported mime should fail");

    assert_eq!(err, "只支持 PNG/JPEG/WebP 图片");
}

#[test]
fn decode_uploaded_profile_material_image_rejects_mime_format_mismatch() {
    let input = upload_input(
        "image/jpeg",
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    );

    let err =
        decode_uploaded_profile_material_image(&input).expect_err("mime mismatch should fail");

    assert_eq!(err, "上传图片 MIME 与真实图片格式不一致");
}

#[test]
fn append_cleanup_error_keeps_secondary_failure_visible() {
    assert_eq!(
        append_cleanup_error("主错误".to_string(), Some("文件清理失败".to_string())),
        "主错误；后续清理也失败：文件清理失败"
    );
    assert_eq!(append_cleanup_error("主错误".to_string(), None), "主错误");
    assert_eq!(
        append_cleanup_error("主错误".to_string(), Some("  ".to_string())),
        "主错误"
    );
}

#[test]
fn profile_material_file_persistence_keeps_output_and_reports_actual_size() {
    let data_dir = unique_temp_dir("a4n_profile_material_persist");

    let (path, relative_path, file_size) = persist_uploaded_profile_material_image(
        &data_dir,
        7,
        KIND_SCENE,
        42,
        "png",
        b"profile-material",
    )
    .expect("persist profile material");

    assert!(path.is_file());
    assert!(relative_path.starts_with("projects/7/images/profile_materials/scene_42_"));
    assert!(relative_path.ends_with(".png"));
    assert_eq!(file_size, 16);
    let _ = std::fs::remove_dir_all(&data_dir);
}

fn material_source(id: i64, name: &str) -> ProfileMaterialSource {
    ProfileMaterialSource {
        kind: KIND_SCENE,
        profile: ProfileMaterialProfileSummary {
            id,
            project_id: 1,
            name: name.to_string(),
            category: "地点".to_string(),
            importance: "primary".to_string(),
            first_seen_chapter: 1,
            last_seen_chapter: 10,
            primary_text: "空间设定".to_string(),
            secondary_text: "氛围".to_string(),
            updated_at: None,
        },
        content_hash: "hash".to_string(),
    }
}

fn image(id: i64, image_type: &str, metadata_json: Value) -> generated_image::Model {
    image_at(id, image_type, metadata_json, 1, false)
}

fn image_at(
    id: i64,
    image_type: &str,
    metadata_json: Value,
    day: u32,
    is_favorite: bool,
) -> generated_image::Model {
    generated_image::Model {
        id,
        project_id: 1,
        image_type: image_type.to_string(),
        file_path: "images/test.png".to_string(),
        prompt: "prompt".to_string(),
        model: "model".to_string(),
        width: 1024,
        height: 768,
        file_size: None,
        ai_score: None,
        is_favorite,
        chapter_id: None,
        panel_id: None,
        metadata_json: Some(metadata_json),
        created_at: NaiveDate::from_ymd_opt(2026, 1, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    }
}

fn image_summary(id: i64, staleness: &str) -> ProfileMaterialImageSummary {
    ProfileMaterialImageSummary {
        image: to_summary(&image(
            id,
            IMAGE_TYPE_SCENE,
            json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
        )),
        is_primary: false,
        staleness: staleness.to_string(),
        style_template_id: None,
        style_template_name: None,
    }
}

fn upload_input(mime_type: &str, data_base64: &str) -> UploadProfileMaterialImageInput {
    UploadProfileMaterialImageInput {
        project_id: 1,
        profile_kind: KIND_SCENE.to_string(),
        profile_id: 1,
        file_name: "ref.png".to_string(),
        mime_type: mime_type.to_string(),
        data_base64: data_base64.to_string(),
        set_primary: true,
        prompt_snapshot: Some("prompt".to_string()),
        style_template_id: None,
        style_template_name: None,
        style_template_version: None,
        custom_style: None,
    }
}
