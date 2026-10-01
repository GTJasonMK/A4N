use super::candidates::{VideoAssetCandidate, best_profile_material_image};
use super::projection::video_library_image_entry;
use super::relevance::{is_chapter_video_material, profile_relevance};
use super::validation::available_video_image_ids;
use super::{
    SELECTION_REASON_PROFILE_FAVORITE, SELECTION_REASON_PROFILE_PRIMARY,
    is_video_reference_image_candidate, resolve_video_reference_image_project_ids,
};
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use a4n_domain::media::GeneratedImageRecord;
use serde_json::json;

#[test]
fn chapter_video_material_accepts_explicit_material_image() {
    let metadata = json!({
        "chapter_video_material": true,
        "generation_context": { "source": "portrait_generation" }
    });

    assert!(is_chapter_video_material(Some(&metadata)));
}

#[test]
fn available_video_image_ids_rejects_invalid_extra_id_instead_of_filtering() {
    let assets = json!([{"image_id": 9, "role": "portrait"}]);
    let err =
        available_video_image_ids(&assets, &[4, 0]).expect_err("invalid extra id should fail");

    assert!(
        err.to_string()
            .contains("reference_image_ids[1] 必须为正整数")
    );
}

#[test]
fn available_video_image_ids_rejects_invalid_asset_entry_instead_of_filtering() {
    let assets = json!([{"image_id": 9}, {"image_id": -1}]);
    let err = available_video_image_ids(&assets, &[4])
        .expect_err("invalid available asset id should fail");

    assert!(
        err.to_string()
            .contains("available_assets[1].image_id 必须为正整数")
    );
}

#[test]
fn profile_relevance_prioritizes_name_match_over_chapter_range() {
    assert_eq!(
        profile_relevance("玄铁匣", &json!(["旧匣"]), 1, 3, 8, "他取出旧匣。"),
        Some(0)
    );
    assert_eq!(
        profile_relevance("偏殿", &json!([]), 3, 9, 8, "无人提到此地。"),
        Some(1)
    );
    assert_eq!(
        profile_relevance("竹林", &json!([]), 1, 2, 8, "无人提到此地。"),
        None
    );
}

#[test]
fn best_profile_material_prefers_primary_then_favorite() {
    let mut first = fixed_test_image(1, false);
    let mut favorite = fixed_test_image(2, true);
    favorite.created_at = chrono::NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();

    let images = vec![first.clone(), favorite.clone()];
    let (image, reason, _, primary) = best_profile_material_image(&images, Some(1)).unwrap();
    assert_eq!(image.id, 1);
    assert_eq!(reason, SELECTION_REASON_PROFILE_PRIMARY);
    assert!(primary);

    first.is_favorite = false;
    let images = vec![first, favorite];
    let (image, reason, _, primary) = best_profile_material_image(&images, None).unwrap();
    assert_eq!(image.id, 2);
    assert_eq!(reason, SELECTION_REASON_PROFILE_FAVORITE);
    assert!(!primary);
}

#[test]
fn video_library_image_entry_includes_display_order_and_label() {
    let image = fixed_test_image(42, false);
    let candidate = VideoAssetCandidate::new(image, "manual_library", 1, 0, 0);
    let entry = video_library_image_entry(&candidate, 3);

    assert_eq!(entry["image_id"], json!(42));
    assert_eq!(entry["display_order"], json!(3));
    assert_eq!(entry["display_label"], json!("参考图 3"));
}

#[test]
fn video_reference_candidate_keeps_original_scope_rules() {
    assert!(is_video_reference_image_candidate(
        &reference_test_image(Some(7), "scene"),
        7
    ));
    assert!(is_video_reference_image_candidate(
        &reference_test_image(None, "scene"),
        7
    ));
    let mut reusable = reference_test_image(Some(9), IMAGE_TYPE_SCENE_MATERIAL);
    reusable.metadata_json = Some(json!({"profile_kind": "scene", "profile_id": 3}));
    assert!(is_video_reference_image_candidate(&reusable, 7));
    assert!(!is_video_reference_image_candidate(
        &reference_test_image(Some(9), "scene"),
        7
    ));
}

#[test]
fn video_reference_project_scope_is_validated_and_deduplicated() {
    assert!(
        resolve_video_reference_image_project_ids(Vec::new(), &[20, 10])
            .unwrap_err()
            .contains("image_project_ids 不能为空")
    );
    assert_eq!(
        resolve_video_reference_image_project_ids(vec![20, 10, 20], &[20, 10]).unwrap(),
        vec![10, 20]
    );
    assert!(
        resolve_video_reference_image_project_ids(vec![10, 0], &[10])
            .unwrap_err()
            .contains("image_project_ids[1] 必须为正整数")
    );
    assert!(
        resolve_video_reference_image_project_ids(vec![10, 99], &[10, 20])
            .unwrap_err()
            .contains("不属于当前视频项目允许的素材项目：99")
    );
}

fn fixed_test_image(id: i64, is_favorite: bool) -> GeneratedImageRecord {
    GeneratedImageRecord {
        id,
        project_id: 1,
        image_type: IMAGE_TYPE_OBJECT_MATERIAL.to_string(),
        file_path: "images/material.png".to_string(),
        prompt: "material".to_string(),
        model: "image-model".to_string(),
        width: 512,
        height: 512,
        file_size: None,
        ai_score: None,
        is_favorite,
        chapter_id: None,
        panel_id: None,
        metadata_json: Some(json!({"profile_kind": "object", "profile_id": 7})),
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 5, 3)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
    }
}

fn reference_test_image(chapter_id: Option<i64>, image_type: &str) -> GeneratedImageRecord {
    let mut image = fixed_test_image(1, false);
    image.chapter_id = chapter_id;
    image.image_type = image_type.to_string();
    image.metadata_json = None;
    image
}
