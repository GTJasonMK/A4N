use super::*;
use crate::project::{ImportMediaPayload, MediaFileEntry, media_file_key};
use a4n_core::models::ai::generated_image;

#[test]
fn validate_media_manifest_rejects_generated_image_unknown_owner_field() {
    let (package, payload) = generated_image_media_fixture("thumbnail_path", b"abc", Some(3));

    let error = validate_media_manifest(&package, &payload)
        .expect_err("generated image media entry must use file_path");

    assert!(error.contains("生成图片媒体文件字段无效"));
}

#[test]
fn validate_media_manifest_rejects_duplicate_owner_field() {
    let (mut package, payload) = generated_image_media_fixture("file_path", b"abc", Some(3));
    package.media_files.push(package.media_files[0].clone());

    let error = validate_media_manifest(&package, &payload)
        .expect_err("owner and field must identify one media entry");

    assert!(error.contains("owner/field 重复"));
}

#[test]
fn validate_media_manifest_rejects_empty_generated_image_bytes() {
    let (package, payload) = generated_image_media_fixture("file_path", b"", Some(0));

    let error = validate_media_manifest(&package, &payload)
        .expect_err("packaged generated image bytes must not be empty");

    assert!(error.contains("媒体文件为空"));
}

#[test]
fn validate_media_manifest_requires_generated_image_owner_entry() {
    let mut package = package_with_derived_project();
    package.generated_images = vec![generated_image(30, None)];

    let error = validate_media_manifest(&package, &empty_media_payload())
        .expect_err("every generated image path must have an owner entry");

    assert!(error.contains("媒体 owner 缺少文件包条目"));
}

#[test]
fn validate_media_manifest_rejects_generated_image_file_size_mismatch() {
    let (package, payload) = generated_image_media_fixture("file_path", b"abc", Some(4));

    let error = validate_media_manifest(&package, &payload)
        .expect_err("generated image bytes must match the recorded file size");

    assert!(error.contains("生成图片文件大小与清单不一致"));
}

#[test]
fn validate_media_manifest_rejects_payload_bytes_outside_manifest() {
    let (package, mut payload) = generated_image_media_fixture("file_path", b"abc", Some(3));
    payload
        .bytes
        .insert("media/unlisted.bin".to_string(), vec![1]);

    let error = validate_media_manifest(&package, &payload)
        .expect_err("payload bytes must exactly match the manifest");

    assert!(error.contains("payload 包含清单外字节"));
}

fn generated_image_media_fixture(
    field: &str,
    bytes: &[u8],
    file_size: Option<i64>,
) -> (ProjectFamilyPackage, ImportMediaPayload) {
    let mut package = package_with_derived_project();
    let image = generated_image(30, file_size);
    let package_path =
        family_media_package_path("generated_image", image.id, field, &image.file_path)
            .expect("fixture path has an extension");
    let entry = MediaFileEntry {
        owner_kind: "generated_image".to_string(),
        owner_id: image.id,
        field: field.to_string(),
        original_path: image.file_path.clone(),
        package_path: package_path.clone(),
    };
    package.generated_images = vec![image];
    package.media_files = vec![entry.clone()];
    (
        package,
        ImportMediaPayload {
            entries: HashMap::from([(
                media_file_key(&entry.owner_kind, entry.owner_id, &entry.field),
                entry,
            )]),
            bytes: HashMap::from([(package_path, bytes.to_vec())]),
        },
    )
}

fn generated_image(id: i64, file_size: Option<i64>) -> generated_image::Model {
    generated_image::Model {
        id,
        project_id: 2,
        image_type: "manga_page".to_string(),
        file_path: format!("images/{id}.png"),
        prompt: "full page".to_string(),
        model: "test".to_string(),
        width: 1024,
        height: 1536,
        file_size,
        ai_score: None,
        is_favorite: false,
        chapter_id: None,
        panel_id: None,
        metadata_json: None,
        created_at: dt(),
    }
}
