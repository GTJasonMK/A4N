use super::{
    CreateGeneratedImageRecordDraftInput, IMAGE_PUBLICATION_STATUS_PENDING,
    IMAGE_PUBLICATION_STATUS_PUBLISHED, IMAGE_PUBLICATION_STATUS_RECLAIMED,
    IMAGE_PUBLICATION_STATUS_RECLAIMING, create_image_publication_intent,
    image_publications_for_recovery, mark_image_publication_reclaimed,
    mark_image_publication_reclaiming, normalize_sha256, project_scoped_path,
    publish_image_publication, required_publication_id,
};
use crate::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{ai::generated_image, media::image_publication};
use a4n_domain::media::{CreateImagePublicationIntentInput, ImagePublicationFileFacts};
use a4n_migration::test_support::connect_test_database;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

#[test]
fn publication_paths_are_project_scoped_and_file_like() {
    assert_eq!(
        project_scoped_path(7, "projects/7/images/a.png", "最终路径").unwrap(),
        "projects/7/images/a.png"
    );
    for path in [
        "projects/8/images/a.png",
        "projects/7/../8/a.png",
        "projects/7\\images\\a.png",
        "projects/7/",
    ] {
        assert!(project_scoped_path(7, path, "最终路径").is_err(), "{path}");
    }
}

#[test]
fn publication_identity_and_hash_are_normalized_at_the_boundary() {
    assert_eq!(
        required_publication_id(" image-publication-1 ").unwrap(),
        "image-publication-1"
    );
    assert_eq!(normalize_sha256(&"A".repeat(64)).unwrap(), "a".repeat(64));
    assert!(required_publication_id("image/publication").is_err());
    assert!(normalize_sha256("not-a-hash").is_err());
}

#[tokio::test]
async fn successful_publication_replay_repairs_missing_image_without_duplicate() {
    let db = connect_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = crate::auth::create_user(
        &db,
        &format!("publication_replay_{suffix}"),
        &format!("publication_replay_{suffix}@example.invalid"),
        "publication-test-password",
        "user",
    )
    .await
    .expect("publication replay test user must be created");
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: "publication replay".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("publication replay test project must be created");
    let publication_id = format!("publication-replay-{suffix}");
    let expected_sha256 = "b".repeat(64);
    create_test_publication_intent(&db, user.id, project.id, &publication_id, &expected_sha256)
        .await;

    let first = publish_image_publication(
        &db,
        &publication_id,
        ImagePublicationFileFacts {
            file_size: 4,
            sha256: expected_sha256.clone(),
        },
    )
    .await
    .expect("first publication must succeed");
    generated_image::Entity::delete_by_id(first.id)
        .exec(&db)
        .await
        .expect("test must be able to remove the published image");
    let repaired = publish_image_publication(
        &db,
        &publication_id,
        ImagePublicationFileFacts {
            file_size: 4,
            sha256: expected_sha256.clone(),
        },
    )
    .await
    .expect("publication replay must rebuild a missing image");
    let replayed = crate::video::read_models::publish_image_publication(
        &db,
        &publication_id,
        ImagePublicationFileFacts {
            file_size: 4,
            sha256: expected_sha256,
        },
    )
    .await
    .expect("successful publication replay must return the repaired image");

    assert_ne!(repaired.id, first.id);
    assert_eq!(replayed.id, repaired.id);
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .all(&db)
            .await
            .expect("published image lookup must succeed")
            .len(),
        1
    );
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .one(&db)
        .await
        .expect("publication lookup must succeed")
        .expect("publication must remain durable");
    assert_eq!(publication.image_id, Some(repaired.id));
    assert_eq!(publication.version, 3);
    let recovered = image_publications_for_recovery(&db, 10, None)
        .await
        .expect("published publication recovery lookup must succeed");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, IMAGE_PUBLICATION_STATUS_PUBLISHED);
}

#[tokio::test]
async fn publication_failure_matrix_preserves_intent_and_blocks_stale_writer() {
    let db = connect_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = crate::auth::create_user(
        &db,
        &format!("publication_{suffix}"),
        &format!("publication_{suffix}@example.invalid"),
        "publication-test-password",
        "user",
    )
    .await
    .expect("publication test user must be created");
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: "publication failure matrix".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("publication test project must be created");
    let publication_id = format!("publication-matrix-{suffix}");
    let expected_sha256 = "a".repeat(64);
    let publication = create_image_publication_intent(
        &db,
        CreateImagePublicationIntentInput {
            publication_id: publication_id.clone(),
            project_id: project.id,
            requested_by_user_id: user.id,
            staging_path: format!("projects/{}/images/.staging/{suffix}.png", project.id),
            final_path: format!("projects/{}/images/{suffix}.png", project.id),
            image: CreateGeneratedImageRecordDraftInput {
                project_id: project.id,
                image_type: "imported".to_string(),
                file_path: format!("projects/{}/images/{suffix}.png", project.id),
                prompt: "publication failure matrix".to_string(),
                model: "test".to_string(),
                width: 1,
                height: 1,
                file_size: Some(4),
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: None,
            },
            expected_file_size: 4,
            expected_sha256: expected_sha256.clone(),
            allow_pending_replacement: false,
        },
    )
    .await
    .expect("publication intent must be durable before file publication");
    assert_eq!(publication.status, IMAGE_PUBLICATION_STATUS_PENDING);
    assert_eq!(publication.version, 1);

    let failed_publish = publish_image_publication(
        &db,
        &publication_id,
        ImagePublicationFileFacts {
            file_size: 3,
            sha256: expected_sha256.clone(),
        },
    )
    .await
    .expect_err("mismatched file facts must not publish an image");
    assert!(
        failed_publish
            .to_string()
            .contains("文件事实与发布意图不一致")
    );
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .expect("failed publication image lookup must succeed")
            .is_none()
    );
    assert_eq!(
        image_publications_for_recovery(&db, 10, None)
            .await
            .expect("pending publication recovery lookup must succeed")
            .len(),
        1
    );

    let reclaiming = mark_image_publication_reclaiming(
        &db,
        &publication_id,
        publication.version,
        "文件事实不匹配".to_string(),
    )
    .await
    .expect("pending publication must claim reclaiming state");
    assert_eq!(reclaiming.status, IMAGE_PUBLICATION_STATUS_RECLAIMING);
    assert_eq!(reclaiming.version, 2);

    let replayed_reclaim = mark_image_publication_reclaiming(
        &db,
        &publication_id,
        publication.version,
        "重复恢复".to_string(),
    )
    .await
    .expect("reclaim claim replay must be idempotent");
    assert_eq!(replayed_reclaim, reclaiming);

    let stale_publish = publish_image_publication(
        &db,
        &publication_id,
        ImagePublicationFileFacts {
            file_size: 4,
            sha256: expected_sha256,
        },
    )
    .await
    .expect_err("a reclaimed publication must reject a late publisher");
    assert!(stale_publish.to_string().contains("发布图片"));

    let stale_reclaim_completion =
        mark_image_publication_reclaimed(&db, &publication_id, publication.version, None)
            .await
            .expect_err("reclaim completion must require the claimed version");
    assert!(
        stale_reclaim_completion
            .to_string()
            .contains("完成图片回收")
    );

    let reclaimed = mark_image_publication_reclaimed(
        &db,
        &publication_id,
        reclaiming.version,
        Some("文件已由 cleanup owner 处理".to_string()),
    )
    .await
    .expect("claimed publication must reach reclaimed state");
    assert_eq!(reclaimed.status, IMAGE_PUBLICATION_STATUS_RECLAIMED);
    assert_eq!(reclaimed.version, 3);

    let replayed_completion =
        mark_image_publication_reclaimed(&db, &publication_id, reclaiming.version, None)
            .await
            .expect("reclaimed terminal replay must be idempotent");
    assert_eq!(replayed_completion, reclaimed);
    assert!(
        image_publications_for_recovery(&db, 10, None)
            .await
            .expect("recovery lookup after reclaim must succeed")
            .is_empty()
    );
}

async fn create_test_publication_intent(
    db: &impl crate::DbExecutor,
    user_id: i64,
    project_id: i64,
    publication_id: &str,
    expected_sha256: &str,
) -> image_publication::Model {
    let staging_path = format!("projects/{project_id}/images/.staging/{publication_id}.png");
    let final_path = format!("projects/{project_id}/images/{publication_id}.png");
    create_image_publication_intent(
        db,
        CreateImagePublicationIntentInput {
            publication_id: publication_id.to_string(),
            project_id,
            requested_by_user_id: user_id,
            staging_path,
            final_path: final_path.clone(),
            image: CreateGeneratedImageRecordDraftInput {
                project_id,
                image_type: "imported".to_string(),
                file_path: final_path,
                prompt: "publication replay".to_string(),
                model: "test".to_string(),
                width: 1,
                height: 1,
                file_size: Some(4),
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: None,
            },
            expected_file_size: 4,
            expected_sha256: expected_sha256.to_string(),
            allow_pending_replacement: false,
        },
    )
    .await
    .expect("test publication intent must be created")
}
