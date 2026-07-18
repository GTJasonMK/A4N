use super::*;
use a4n_core::models::task::cleanup_outbox;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

#[test]
fn project_deletion_payload_sorts_deduplicates_and_drops_blank_paths() {
    let payload = ProjectDeletionCleanupPayload::new(
        7,
        vec![
            "b.mp4".to_string(),
            "".to_string(),
            "  ".to_string(),
            "a.png".to_string(),
            "b.mp4".to_string(),
        ],
    );
    assert_eq!(payload.project_id, 7);
    assert_eq!(payload.media_paths, vec!["a.png", "b.mp4"]);
}

#[tokio::test]
async fn expired_cleanup_lease_is_fenced_and_reclaimed() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_project_deletion_cleanup_intent(
        &db,
        format!("test-project-delete-{project_id}"),
        ProjectDeletionCleanupPayload::new(project_id, vec!["a.png".to_string()]),
    )
    .await
    .unwrap();
    let first = claim_project_deletion_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.record.attempt_count, 1);
    assert!(
        claim_project_deletion_cleanup(&db, Some(project_id))
            .await
            .unwrap()
            .is_none()
    );

    let mut expired: cleanup_outbox::ActiveModel = first.record.clone().into();
    expired.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    expired.update(&db).await.unwrap();
    let second = claim_project_deletion_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.record.attempt_count, 2);
    assert_ne!(first.record.lease_token, second.record.lease_token);
    let old_token = first.record.lease_token.as_deref().unwrap();
    assert!(
        complete_claimed_cleanup(&db, inserted.id, old_token)
            .await
            .is_err()
    );
    retry_claimed_cleanup(
        &db,
        inserted.id,
        second.record.lease_token.as_deref().unwrap(),
        "transient cleanup failure".to_string(),
        Duration::zero(),
    )
    .await
    .unwrap();
    let pending = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.status, CLEANUP_STATUS_PENDING);
    assert_eq!(
        pending.last_error.as_deref(),
        Some("transient cleanup failure")
    );
    assert!(pending.lease_token.is_none());

    let third = claim_project_deletion_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(third.record.attempt_count, 3);
    complete_claimed_cleanup(
        &db,
        inserted.id,
        third.record.lease_token.as_deref().unwrap(),
    )
    .await
    .unwrap();
    let completed = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, CLEANUP_STATUS_COMPLETED);
    assert!(completed.completed_at.is_some());
    assert!(completed.lease_token.is_none());
}

#[tokio::test]
async fn invalid_payload_is_failed_without_starving_later_cleanup() {
    let db = setup_db().await;
    let seed = Utc::now().timestamp_micros();
    let invalid = insert_project_deletion_cleanup_intent(
        &db,
        format!("test-invalid-project-delete-{seed}"),
        ProjectDeletionCleanupPayload::new(seed, Vec::new()),
    )
    .await
    .unwrap();
    let mut invalid_active: cleanup_outbox::ActiveModel = invalid.clone().into();
    invalid_active.payload_json =
        Set(serde_json::json!({"version": 999, "project_id": seed, "media_paths": []}));
    invalid_active.update(&db).await.unwrap();
    let valid_project_id = seed + 1;
    let valid = insert_project_deletion_cleanup_intent(
        &db,
        format!("test-valid-project-delete-{seed}"),
        ProjectDeletionCleanupPayload::new(valid_project_id, Vec::new()),
    )
    .await
    .unwrap();

    let claimed = claim_project_deletion_cleanup(&db, None)
        .await
        .unwrap()
        .expect("valid cleanup after poison row must still be claimable");
    assert_eq!(claimed.record.id, valid.id);
    let failed = cleanup_outbox::Entity::find_by_id(invalid.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, CLEANUP_STATUS_FAILED);
    assert!(failed.last_error.as_deref().unwrap().contains("合同不一致"));
}
