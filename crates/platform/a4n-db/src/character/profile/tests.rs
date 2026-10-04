use super::*;
use crate::character::{CharacterDbError, create_character_portrait_for_profile};
use a4n_db_models::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
    media::{media_asset, media_binding},
};
use a4n_domain::character::CharacterPortraitCreateInput;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde_json::json;

#[test]
fn initial_profile_editability_has_one_complete_rule() {
    assert!(initial_profile_editable(0, true, false));
    assert!(!initial_profile_editable(0, false, false));
    assert!(!initial_profile_editable(0, true, true));
    assert!(!initial_profile_editable(1, true, false));
}

#[test]
fn apply_current_state_refreshes_profile_summary_fields() {
    let state = json!({
        "explicit_attributes": {
            "gender": "女",
            "age": "二十七",
            "appearance": "黑衣",
            "background": "前朝密探"
        },
        "implicit_attributes": {
            "personality": "谨慎"
        },
        "social_attributes": {}
    });
    let mut active: protagonist_profile::ActiveModel = Default::default();

    apply_profile_current_state(&mut active, state.clone());

    assert_eq!(active.gender, Set(Some("女".to_string())));
    assert_eq!(active.age, Set(Some("二十七".to_string())));
    assert_eq!(active.appearance, Set(Some("黑衣".to_string())));
    assert_eq!(active.background, Set(Some("前朝密探".to_string())));
    assert_eq!(active.personality, Set(Some("谨慎".to_string())));
    assert_eq!(active.current_state, Set(state));
}

#[test]
fn normalize_base_attributes_accepts_canonical_state() {
    let normalized = normalize_base_attributes(json!({
        "explicit_attributes": {
            "gender": "女",
            "appearance": "黑衣"
        },
        "implicit_attributes": {
            "personality": "谨慎"
        },
        "social_attributes": {}
    }))
    .expect("canonical initial state should pass");

    assert_eq!(normalized["explicit_attributes"]["gender"], json!("女"));
    assert_eq!(
        normalized["explicit_attributes"]["appearance"],
        json!("黑衣")
    );
    assert_eq!(
        normalized["implicit_attributes"]["personality"],
        json!("谨慎")
    );
    assert_eq!(normalized["social_attributes"], json!({}));
}

#[test]
fn normalize_base_attributes_rejects_flat_profile_form() {
    let error = normalize_base_attributes(json!({
        "gender": "女",
        "personality": "谨慎"
    }))
    .expect_err("flat create form should fail");

    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message) if message.contains("explicit_attributes")
    ));
}

#[test]
fn normalize_base_attributes_rejects_partial_canonical_state() {
    let error = normalize_base_attributes(json!({
        "explicit_attributes": {}
    }))
    .expect_err("canonical initial state must include all categories");

    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message) if message.contains("implicit_attributes")
    ));
}

#[test]
fn normalize_base_attributes_rejects_unknown_top_level_field() {
    let error = normalize_base_attributes(json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
        "unknown": {}
    }))
    .expect_err("unknown top-level field must fail");

    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message)
            if message == "初始角色属性包含未知顶层字段: unknown"
    ));
}

#[tokio::test]
async fn profile_cascade_preserves_other_portrait_with_shared_path() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "角色档案共享路径删除测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let base_attributes = || {
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        })
    };
    let profile_a = create_profile(&db, project.id, "角色 A", base_attributes())
        .await
        .unwrap();
    let profile_b = create_profile(&db, project.id, "角色 B", base_attributes())
        .await
        .unwrap();
    let shared_path = "projects/shared/portraits/profile-shared.png";
    let portrait_a = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile_a.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: shared_path.to_string(),
            prompt: "portrait A".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let portrait_b = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile_b.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: shared_path.to_string(),
            prompt: "portrait B".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let image_a = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "portrait".to_string(),
            file_path: shared_path.to_string(),
            prompt: "portrait A".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({"portrait_id": portrait_a.id})),
            created_at: None,
        },
    )
    .await
    .unwrap();
    let image_b = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "portrait".to_string(),
            file_path: shared_path.to_string(),
            prompt: "portrait B".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({"portrait_id": portrait_b.id})),
            created_at: None,
        },
    )
    .await
    .unwrap();
    let asset = crate::canvas::ensure_canvas_image_media_asset_record(
        &db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image: crate::video::read_models::generated_image_by_id(&db, image_a.id)
                .await
                .unwrap()
                .unwrap(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();
    crate::canvas::ensure_canvas_image_media_asset_record(
        &db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image: crate::video::read_models::generated_image_by_id(&db, image_b.id)
                .await
                .unwrap()
                .unwrap(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();

    let summary = delete_profile_cascade_transaction(&db, profile_a.id)
        .await
        .unwrap();

    assert!(summary.portrait_paths.is_empty());
    assert!(
        protagonist_profile::Entity::find_by_id(profile_a.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(image_a.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        protagonist_profile::Entity::find_by_id(profile_b.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        character_portrait::Entity::find_by_id(portrait_b.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        generated_image::Entity::find_by_id(image_b.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(image_a.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(image_b.id))
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert!(
        media_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn profile_batch_cascade_deletes_profiles_in_one_scope() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "角色档案批量删除测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let base_attributes = || {
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        })
    };
    let profile_a = create_profile(&db, project.id, "角色批量 A", base_attributes())
        .await
        .unwrap();
    let profile_b = create_profile(&db, project.id, "角色批量 B", base_attributes())
        .await
        .unwrap();

    delete_profiles_cascade_in(&db, project.id, &[profile_a.id, profile_b.id])
        .await
        .expect("profiles in one project scope should be deleted together");

    assert_eq!(
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::Id.is_in([profile_a.id, profile_b.id]))
            .count(&db)
            .await
            .unwrap(),
        0
    );
}
