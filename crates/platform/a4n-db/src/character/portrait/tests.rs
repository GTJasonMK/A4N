use super::*;
use a4n_db_models::models::{
    character::protagonist_profile, media::media_binding, project::novel_project,
};
use a4n_domain::character::CharacterPortraitCreateInput;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

#[tokio::test]
async fn portrait_project_index_is_installed_by_schema_migrations() {
    let db = setup_db().await;
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT indexdef FROM pg_indexes WHERE schemaname = current_schema() AND indexname = 'idx_character_portrait_project_id'".to_string(),
        ))
        .await
        .unwrap()
        .expect("portrait project index must exist");
    let indexdef: String = row.try_get("", "indexdef").unwrap();

    assert!(indexdef.contains("character_portrait"));
    assert!(indexdef.contains("project_id"));
}

#[tokio::test]
async fn portrait_primary_profile_index_is_installed_by_schema_migrations() {
    let db = setup_db().await;
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT indexdef FROM pg_indexes WHERE schemaname = current_schema() AND indexname = 'idx_character_portrait_primary_profile'".to_string(),
        ))
        .await
        .unwrap()
        .expect("portrait primary profile index must exist");
    let indexdef: String = row.try_get("", "indexdef").unwrap();

    assert!(indexdef.contains("character_portrait"));
    assert!(indexdef.contains("profile_id"));
    assert!(indexdef.contains("WHERE (is_primary = true)"));
}

fn empty_profile_attributes() -> Value {
    json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {}
    })
}

async fn project_and_profile(
    db: &impl crate::DbExecutor,
    title: &str,
) -> (novel_project::Model, protagonist_profile::Model) {
    let project = crate::project::create_canvas_project_record(
        db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: title.to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let profile =
        crate::character::create_profile(db, project.id, "测试角色", empty_profile_attributes())
            .await
            .unwrap();
    (project, profile)
}

#[tokio::test]
async fn missing_portrait_profiles_returns_only_profiles_without_portraits() {
    let db = setup_db().await;
    let (project, covered_profile) = project_and_profile(&db, "缺失立绘批量检查").await;
    let missing_profile =
        crate::character::create_profile(&db, project.id, "待生成角色", empty_profile_attributes())
            .await
            .unwrap();
    create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: covered_profile.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: "portraits/covered.png".to_string(),
            prompt: "covered".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();

    let missing = missing_portrait_profiles(&db, project.id).await.unwrap();
    assert_eq!(
        missing.iter().map(|profile| profile.id).collect::<Vec<_>>(),
        vec![missing_profile.id]
    );
}

async fn primary_ids(db: &impl crate::DbExecutor, profile_id: i64) -> Vec<i64> {
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(true))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|portrait| portrait.id)
        .collect()
}

fn image_with_portrait_metadata(metadata_json: Option<Value>) -> generated_image::Model {
    generated_image::Model {
        id: 1,
        project_id: 1,
        image_type: "portrait".to_string(),
        file_path: "portrait.png".to_string(),
        prompt: "portrait".to_string(),
        model: "test".to_string(),
        width: 512,
        height: 512,
        file_size: Some(1024),
        ai_score: None,
        is_favorite: false,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        created_at: Utc::now().naive_utc(),
    }
}

#[test]
fn portrait_record_error_preserves_classification_and_database_source() {
    let not_found = CharacterPortraitRecordError::NotFound("角色档案不存在: 7".to_string());
    assert!(matches!(
        not_found,
        CharacterPortraitRecordError::NotFound(message)
            if message == "角色档案不存在: 7"
    ));

    let invalid =
        CharacterPortraitRecordError::InvalidArgument("角色档案 7 不属于项目 8".to_string());
    assert!(matches!(
        invalid,
        CharacterPortraitRecordError::InvalidArgument(message)
            if message == "角色档案 7 不属于项目 8"
    ));

    let database =
        CharacterPortraitRecordError::Database(DbErr::Custom("portrait query failed".to_string()));
    assert!(matches!(
        database,
        CharacterPortraitRecordError::Database(DbErr::Custom(message))
            if message == "portrait query failed"
    ));
}

#[test]
fn portrait_image_metadata_prevents_cross_portrait_deletion() {
    assert!(generated_image_matches_portrait(
        &image_with_portrait_metadata(Some(json!({"portrait_id": 7}))),
        7
    ));
    assert!(!generated_image_matches_portrait(
        &image_with_portrait_metadata(Some(json!({"portrait_id": 8}))),
        7
    ));
    assert!(generated_image_matches_portrait(
        &image_with_portrait_metadata(None),
        7
    ));
    assert!(generated_image_matches_any_portrait(
        &image_with_portrait_metadata(Some(json!({"character_portrait_id": 8}))),
        &[7, 8]
    ));
}

#[tokio::test]
async fn create_portrait_rejects_profile_from_another_project_without_insert() {
    let db = setup_db().await;
    let (profile_project, profile) = project_and_profile(&db, "立绘档案项目").await;
    let (target_project, _) = project_and_profile(&db, "立绘目标项目").await;

    let error = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: target_project.id,
            portrait_type: "full_body".to_string(),
            image_path: "portraits/lin.png".to_string(),
            prompt: "portrait prompt".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .expect_err("portrait project must match profile project");

    assert_ne!(profile_project.id, target_project.id);
    assert!(matches!(
        error,
        CharacterPortraitRecordError::InvalidArgument(message)
            if message.contains(&format!("不属于项目 {}", target_project.id))
    ));
    assert_eq!(
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.eq(profile.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn first_portrait_is_primary_and_later_portraits_are_not() {
    let db = setup_db().await;
    let (project, profile) = project_and_profile(&db, "立绘主图初始化").await;

    let first = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: "portraits/1.png".to_string(),
            prompt: "portrait one".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let second = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "half_body".to_string(),
            image_path: "portraits/2.png".to_string(),
            prompt: "portrait two".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();

    assert!(first.is_primary);
    assert!(!second.is_primary);
    assert_eq!(primary_ids(&db, profile.id).await, vec![first.id]);
}

#[tokio::test]
async fn set_primary_portrait_leaves_exactly_one_primary() {
    let db = setup_db().await;
    let (project, profile) = project_and_profile(&db, "立绘主图切换").await;
    let first = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: "portraits/1.png".to_string(),
            prompt: "portrait one".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let second = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "half_body".to_string(),
            image_path: "portraits/2.png".to_string(),
            prompt: "portrait two".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();

    set_primary_character_portrait_record(&db, second.id)
        .await
        .unwrap();

    assert_ne!(first.id, second.id);
    assert_eq!(primary_ids(&db, profile.id).await, vec![second.id]);
}

#[tokio::test]
async fn portrait_rollback_is_transactional_and_project_scoped() {
    let db = setup_db().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "立绘回滚目标项目".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let other_project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "立绘回滚隔离项目".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let profile =
        crate::character::create_profile(&db, project.id, "测试角色", empty_profile_attributes())
            .await
            .unwrap();
    let image_path = "projects/shared/portraits/same.png";
    let portrait = create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: image_path.to_string(),
            prompt: "portrait".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let target_image = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "portrait".to_string(),
            file_path: image_path.to_string(),
            prompt: "portrait".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({
                "portrait_id": portrait.id,
                "profile_id": profile.id,
            })),
            created_at: None,
        },
    )
    .await
    .unwrap();
    let other_image = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: other_project.id,
            image_type: "portrait".to_string(),
            file_path: image_path.to_string(),
            prompt: "other portrait".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({"portrait_id": i64::MAX})),
            created_at: None,
        },
    )
    .await
    .unwrap();
    crate::canvas::ensure_canvas_image_media_asset_record(
        &db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image: crate::video::read_models::generated_image_by_id(&db, target_image.id)
                .await
                .unwrap()
                .unwrap(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();

    let err =
        rollback_failed_portrait_creation_records(&db, portrait.id, "projects/wrong/portrait.png")
            .await
            .expect_err("path mismatch must not delete portrait data");
    assert!(matches!(
        err,
        CharacterPortraitRollbackError::InvalidArgument(_)
    ));
    assert!(
        generated_image::Entity::find_by_id(target_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    let rollback = rollback_failed_portrait_creation_records(&db, portrait.id, image_path)
        .await
        .unwrap();
    assert!(!rollback.delete_file, "保留 media asset 时不能删除磁盘文件");

    assert!(
        character_portrait::Entity::find_by_id(portrait.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(target_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(target_image.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    let err = rollback_failed_portrait_creation_records(&db, i64::MAX, image_path)
        .await
        .expect_err("missing portrait must not delete same-path images");
    assert!(matches!(err, CharacterPortraitRollbackError::NotFound(_)));
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}
