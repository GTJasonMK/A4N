use super::contracts::{
    parse_file_artifact_cleanup_payload, parse_generation_cleanup_payload,
    validated_project_deletion_cleanup_payload,
};
use super::*;
use a4n_db_models::models::task::cleanup_outbox;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, Set, Statement,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

#[test]
fn project_deletion_payload_sorts_deduplicates_and_drops_blank_paths() {
    let payload = ProjectDeletionCleanupPayload::new(
        7,
        vec![
            "projects/7/b.mp4".to_string(),
            "".to_string(),
            "  ".to_string(),
            "projects/7/a.png".to_string(),
            "projects/7/b.mp4".to_string(),
        ],
    );
    assert_eq!(payload.project_id, 7);
    assert_eq!(
        payload.media_paths,
        vec!["projects/7/a.png", "projects/7/b.mp4"]
    );
}

#[test]
fn project_deletion_payload_rejects_cross_project_and_traversal_paths() {
    assert!(
        validated_project_deletion_cleanup_payload(7, vec!["projects/8/video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_project_deletion_cleanup_payload(
            7,
            vec!["projects/7/../8/video.mp4".to_string()]
        )
        .is_err()
    );
    assert!(
        validated_project_deletion_cleanup_payload(
            7,
            vec!["projects/7/..\\8/video.mp4".to_string()]
        )
        .is_err()
    );
    assert!(
        validated_project_deletion_cleanup_payload(7, vec!["projects/7\\video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_project_deletion_cleanup_payload(7, vec!["projects/7/video.mp4".to_string()])
            .is_ok()
    );
}

#[test]
fn file_artifact_payload_sorts_deduplicates_and_drops_blank_paths() {
    let payload = FileArtifactCleanupPayload::new(
        9,
        vec![
            "projects/9/video.mp4".to_string(),
            " ".to_string(),
            "projects/9/thumb.png".to_string(),
            "projects/9/video.mp4".to_string(),
        ],
    );
    assert_eq!(payload.project_id, 9);
    assert_eq!(
        payload.paths,
        vec!["projects/9/thumb.png", "projects/9/video.mp4"]
    );
}

#[test]
fn file_artifact_payload_rejects_cross_project_and_traversal_paths() {
    assert!(
        validated_file_artifact_cleanup_payload(9, vec!["projects/10/video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_file_artifact_cleanup_payload(9, vec!["projects/9/../10/video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_file_artifact_cleanup_payload(9, vec!["projects/9/..\\10/video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_file_artifact_cleanup_payload(9, vec!["projects/9\\video.mp4".to_string()])
            .is_err()
    );
    assert!(
        validated_file_artifact_cleanup_payload(9, vec!["projects/9/video.mp4".to_string()])
            .is_ok()
    );

    let now = Utc::now().naive_utc();
    let payload_json = serde_json::json!({
        "version": 1,
        "project_id": 9,
        "paths": ["projects/10/video.mp4"]
    });
    let record = cleanup_outbox::Model {
        id: 1,
        dedupe_key: "invalid-file-artifact".to_string(),
        cleanup_kind: FILE_ARTIFACT_CLEANUP_KIND.to_string(),
        project_id: Some(9),
        requested_by_user_id: None,
        task_id: None,
        attempt: None,
        payload_version: 1,
        payload_sha256: crate::generation::canonical_json_sha256(&payload_json).unwrap(),
        payload_json,
        status: CLEANUP_STATUS_PENDING.to_string(),
        attempt_count: 0,
        observation_count: 0,
        next_attempt_at: now,
        lease_token: None,
        lease_expires_at: None,
        last_error: None,
        outcome: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
    };
    assert!(parse_file_artifact_cleanup_payload(&record).is_err());
}

#[test]
fn generation_cleanup_payload_is_typed_and_scope_bound() {
    let now = Utc::now().naive_utc();
    let payload = GenerationProviderJobCleanupPayload::new(
        GenerationProviderJobCleanupOperation::Cancel,
        "video".to_string(),
        "job-1".to_string(),
        "a".repeat(64),
    )
    .unwrap();
    let payload_json = serde_json::to_value(payload).unwrap();
    let mut record = cleanup_outbox::Model {
        id: 2,
        dedupe_key: "generation-provider-cleanup".to_string(),
        cleanup_kind: GENERATION_PROVIDER_JOB_CLEANUP_KIND.to_string(),
        project_id: Some(9),
        requested_by_user_id: Some(3),
        task_id: Some("task-1".to_string()),
        attempt: Some(1),
        payload_version: 1,
        payload_sha256: crate::generation::canonical_json_sha256(&payload_json).unwrap(),
        payload_json,
        status: CLEANUP_STATUS_PENDING.to_string(),
        attempt_count: 0,
        observation_count: 0,
        next_attempt_at: now,
        lease_token: None,
        lease_expires_at: None,
        last_error: None,
        outcome: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
    };
    assert!(matches!(
        parse_generation_cleanup_payload(&record).unwrap(),
        GenerationCleanupPayload::ProviderJob(_)
    ));

    record.requested_by_user_id = None;
    assert!(parse_generation_cleanup_payload(&record).is_err());
}

#[tokio::test]
async fn cleanup_intent_dedupe_is_idempotent_but_rejects_other_payload() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let dedupe_key = format!("test-idempotent-file-artifact-{project_id}");
    let payload = FileArtifactCleanupPayload::new(
        project_id,
        vec![format!("projects/{project_id}/video.mp4")],
    );

    let first = insert_file_artifact_cleanup_intent(&db, dedupe_key.clone(), payload.clone())
        .await
        .unwrap();
    let second = insert_file_artifact_cleanup_intent(&db, dedupe_key.clone(), payload)
        .await
        .unwrap();
    assert_eq!(first.id, second.id);

    let error = insert_file_artifact_cleanup_intent(
        &db,
        dedupe_key,
        FileArtifactCleanupPayload::new(
            project_id,
            vec![format!("projects/{project_id}/other.mp4")],
        ),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("已绑定其他 payload"));
}

#[tokio::test]
async fn expired_cleanup_lease_is_fenced_and_reclaimed() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_project_deletion_cleanup_intent(
        &db,
        format!("test-project-delete-{project_id}"),
        ProjectDeletionCleanupPayload::new(
            project_id,
            vec![format!("projects/{project_id}/a.png")],
        ),
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
async fn completed_cleanup_audit_ttl_prunes_only_expired_rows_in_batches() {
    async fn completed_audit(
        db: &DatabaseConnection,
        project_id: i64,
        age_days: i64,
    ) -> cleanup_outbox::Model {
        let inserted = insert_project_deletion_cleanup_intent(
            db,
            format!("test-cleanup-audit-ttl-{project_id}"),
            ProjectDeletionCleanupPayload::new(project_id, Vec::new()),
        )
        .await
        .unwrap();
        let claim = claim_project_deletion_cleanup(db, Some(project_id))
            .await
            .unwrap()
            .unwrap();
        complete_claimed_cleanup(
            db,
            inserted.id,
            claim.record.lease_token.as_deref().unwrap(),
        )
        .await
        .unwrap();
        let mut active: cleanup_outbox::ActiveModel =
            cleanup_outbox::Entity::find_by_id(inserted.id)
                .one(db)
                .await
                .unwrap()
                .unwrap()
                .into();
        let completed_at = Utc::now().naive_utc() - Duration::days(age_days);
        active.completed_at = Set(Some(completed_at));
        active.updated_at = Set(completed_at);
        active.update(db).await.unwrap()
    }

    let db = setup_db().await;
    let seed = Utc::now().timestamp_micros();
    let oldest = completed_audit(&db, seed, 32).await;
    let expired = completed_audit(&db, seed + 1, 31).await;
    let recent = completed_audit(&db, seed + 2, 29).await;

    assert_eq!(purge_completed_cleanup_audits(&db, 1).await.unwrap(), 1);
    assert!(
        cleanup_outbox::Entity::find_by_id(oldest.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        cleanup_outbox::Entity::find_by_id(expired.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    assert_eq!(purge_completed_cleanup_audits(&db, 100).await.unwrap(), 1);
    assert!(
        cleanup_outbox::Entity::find_by_id(expired.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        cleanup_outbox::Entity::find_by_id(recent.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn generation_cleanup_retry_uses_bounded_schedule_and_redacts_error() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_generation_provider_job_cleanup_intent(
        &db,
        format!("test-generation-cleanup-retry-{project_id}"),
        &GenerationCleanupScope {
            requested_by_user_id: 1,
            project_id: Some(project_id),
            task_id: format!("generation-retry-{project_id}"),
            attempt: 1,
        },
        GenerationProviderJobCleanupPayload::new(
            GenerationProviderJobCleanupOperation::Cancel,
            "video".to_string(),
            "job-retry".to_string(),
            "a".repeat(64),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let delays = [
        Duration::minutes(1),
        Duration::minutes(5),
        Duration::minutes(30),
        Duration::hours(2),
        Duration::hours(12),
    ];
    let mut claim = claim_generation_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();

    for (index, delay) in delays.iter().enumerate() {
        let outcome = retry_or_fail_claimed_generation_cleanup(
            &db,
            inserted.id,
            claim.record.lease_token.as_deref().unwrap(),
            "provider error api_key=secret https://provider.example/jobs/1".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(outcome, GenerationCleanupRetryOutcome::Scheduled);
        let row = cleanup_outbox::Entity::find_by_id(inserted.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, CLEANUP_STATUS_PENDING);
        assert_eq!(row.attempt_count, (index + 1) as i32);
        assert_eq!(row.next_attempt_at - row.updated_at, *delay);
        let last_error = row.last_error.as_deref().unwrap();
        assert!(!last_error.contains("secret"));
        assert!(!last_error.contains("https://provider.example"));
        assert!(last_error.contains("[REDACTED"));

        let mut due: cleanup_outbox::ActiveModel = row.into();
        due.next_attempt_at = Set(Utc::now().naive_utc() - Duration::seconds(1));
        due.update(&db).await.unwrap();
        claim = claim_generation_cleanup(&db, Some(project_id))
            .await
            .unwrap()
            .unwrap();
    }

    let outcome = retry_or_fail_claimed_generation_cleanup(
        &db,
        inserted.id,
        claim.record.lease_token.as_deref().unwrap(),
        "provider error api_key=secret https://provider.example/jobs/1".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerationCleanupRetryOutcome::Exhausted);
    let failed = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, CLEANUP_STATUS_FAILED);
    assert_eq!(failed.attempt_count, 5);
    assert!(failed.lease_token.is_none());
}

#[tokio::test]
async fn generation_reconcile_observation_does_not_consume_retry_attempt() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_generation_provider_job_cleanup_intent(
        &db,
        format!("test-generation-reconcile-observe-{project_id}"),
        &GenerationCleanupScope {
            requested_by_user_id: 1,
            project_id: Some(project_id),
            task_id: format!("generation-reconcile-observe-{project_id}"),
            attempt: 1,
        },
        GenerationProviderJobCleanupPayload::new(
            GenerationProviderJobCleanupOperation::Reconcile,
            "openai-compatible-video".to_string(),
            "job-observe".to_string(),
            "b".repeat(64),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let claim = claim_generation_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();

    let outcome = observe_claimed_generation_reconcile(
        &db,
        inserted.id,
        claim.record.lease_token.as_deref().unwrap(),
        Duration::seconds(42),
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerationCleanupObservationOutcome::Rescheduled);
    let row = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, CLEANUP_STATUS_PENDING);
    assert_eq!(row.attempt_count, 0);
    assert_eq!(row.observation_count, 1);
    assert_eq!(row.next_attempt_at - row.updated_at, Duration::seconds(42));
    assert!(row.lease_token.is_none());
}

#[tokio::test]
async fn completed_reconcile_atomically_schedules_one_delete_intent() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_generation_provider_job_cleanup_intent(
        &db,
        format!("test-generation-reconcile-delete-{project_id}"),
        &GenerationCleanupScope {
            requested_by_user_id: 1,
            project_id: Some(project_id),
            task_id: format!("generation-reconcile-delete-{project_id}"),
            attempt: 1,
        },
        GenerationProviderJobCleanupPayload::new(
            GenerationProviderJobCleanupOperation::Reconcile,
            "openai-compatible-video".to_string(),
            "job-delete".to_string(),
            "c".repeat(64),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let claim = claim_generation_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();
    let lease_token = claim.record.lease_token.as_deref().unwrap();

    let delete = schedule_provider_delete_from_reconcile(&db, inserted.id, lease_token)
        .await
        .unwrap();
    let reconcile = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reconcile.status, CLEANUP_STATUS_COMPLETED);
    assert_eq!(reconcile.outcome.as_deref(), Some("delete_scheduled"));
    let GenerationCleanupPayload::ProviderJob(delete_payload) =
        parse_generation_cleanup_payload(&delete).unwrap()
    else {
        panic!("delete intent must keep provider payload");
    };
    assert_eq!(
        delete_payload.operation,
        GenerationProviderJobCleanupOperation::Delete
    );
    assert_eq!(delete_payload.provider_job_id, "job-delete");
    assert!(
        schedule_provider_delete_from_reconcile(&db, inserted.id, lease_token)
            .await
            .is_err()
    );
    assert_eq!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::DedupeKey.eq(delete.dedupe_key))
            .all(&db)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn generation_reconcile_deadline_fails_without_consuming_retry_attempt() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let inserted = insert_generation_provider_job_cleanup_intent(
        &db,
        format!("test-generation-reconcile-deadline-{project_id}"),
        &GenerationCleanupScope {
            requested_by_user_id: 1,
            project_id: Some(project_id),
            task_id: format!("generation-reconcile-deadline-{project_id}"),
            attempt: 1,
        },
        GenerationProviderJobCleanupPayload::new(
            GenerationProviderJobCleanupOperation::Reconcile,
            "openai-compatible-video".to_string(),
            "job-deadline".to_string(),
            "d".repeat(64),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let mut old: cleanup_outbox::ActiveModel = inserted.clone().into();
    old.created_at = Set(Utc::now().naive_utc() - Duration::days(31));
    old.update(&db).await.unwrap();
    let claim = claim_generation_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .unwrap();

    let outcome = observe_claimed_generation_reconcile(
        &db,
        inserted.id,
        claim.record.lease_token.as_deref().unwrap(),
        Duration::seconds(60),
    )
    .await
    .unwrap();
    assert_eq!(
        outcome,
        GenerationCleanupObservationOutcome::DeadlineExceeded
    );
    let row = cleanup_outbox::Entity::find_by_id(inserted.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, CLEANUP_STATUS_FAILED);
    assert_eq!(row.attempt_count, 0);
    assert_eq!(row.observation_count, 0);
    assert!(row.last_error.unwrap().contains("30 天"));
}

#[tokio::test]
async fn concurrent_cleanup_claim_has_one_owner() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    insert_project_deletion_cleanup_intent(
        &db,
        format!("test-concurrent-project-delete-{project_id}"),
        ProjectDeletionCleanupPayload::new(
            project_id,
            vec![format!("projects/{project_id}/a.png")],
        ),
    )
    .await
    .unwrap();

    let first_db = db.clone();
    let second_db = db.clone();
    let (first, second) = tokio::join!(
        claim_project_deletion_cleanup(&first_db, Some(project_id)),
        claim_project_deletion_cleanup(&second_db, Some(project_id)),
    );
    let first = first.unwrap();
    let second = second.unwrap();

    assert!(first.is_some() ^ second.is_some());
    let claim = first.or(second).unwrap();
    let conflicting_locks = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
SELECT lock.mode
FROM pg_locks AS lock
JOIN pg_class AS relation ON relation.oid = lock.relation
JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
WHERE namespace.nspname = current_schema()
  AND relation.relname = 'project_cleanup_outbox'
  AND lock.pid <> pg_backend_pid()
  AND lock.granted
  AND lock.mode <> 'AccessShareLock'
"#,
        ))
        .await
        .unwrap();
    assert!(
        conflicting_locks.is_empty(),
        "cleanup claim 返回后不得保留 outbox transaction lock"
    );
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM project_cleanup_outbox WHERE id = $1 FOR UPDATE NOWAIT",
        [claim.record.id.into()],
    ))
    .await
    .unwrap()
    .expect("claimed cleanup row should remain lockable after claim commit");
}

#[tokio::test]
async fn file_artifact_cleanup_claim_has_one_owner() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    insert_file_artifact_cleanup_intent(
        &db,
        format!("test-file-artifact-{project_id}"),
        FileArtifactCleanupPayload::new(
            project_id,
            vec![format!("projects/{project_id}/video.mp4")],
        ),
    )
    .await
    .unwrap();

    let first_db = db.clone();
    let second_db = db.clone();
    let (first, second) = tokio::join!(
        claim_file_artifact_cleanup(&first_db, Some(project_id)),
        claim_file_artifact_cleanup(&second_db, Some(project_id)),
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert!(first.is_some() ^ second.is_some());
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
    let invalid_payload =
        serde_json::json!({"version": 999, "project_id": seed, "media_paths": []});
    invalid_active.payload_sha256 =
        Set(crate::generation::canonical_json_sha256(&invalid_payload).unwrap());
    invalid_active.payload_json = Set(invalid_payload);
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

#[tokio::test]
async fn generation_artifact_unbind_intent_is_claimable_and_parseable() {
    let db = setup_db().await;
    let project_id = Utc::now().timestamp_micros();
    let artifact_id = "0123456789abcdef0123456789abcdef".to_string();
    let revision = "e".repeat(64);
    let inserted = insert_generation_artifact_unbind_intent(
        &db,
        format!("test-generation-unbind-{project_id}"),
        &GenerationCleanupScope {
            requested_by_user_id: 1,
            project_id: Some(project_id),
            task_id: format!("generation-unbind-{project_id}"),
            attempt: 1,
        },
        GenerationArtifactUnbindPayload {
            version: 1,
            artifact_id: artifact_id.clone(),
            revision: revision.clone(),
        },
    )
    .await
    .unwrap();
    let claim = claim_generation_cleanup(&db, Some(project_id))
        .await
        .unwrap()
        .expect("unbind intent must be claimable");
    assert_eq!(claim.record.id, inserted.id);
    match claim.payload {
        GenerationCleanupPayload::ArtifactUnbind(payload) => {
            assert_eq!(payload.artifact_id, artifact_id);
            assert_eq!(payload.revision, revision);
        }
        other => panic!("expected ArtifactUnbind, got {other:?}"),
    }
    complete_claimed_generation_cleanup(
        &db,
        claim.record.id,
        claim.record.lease_token.as_deref().unwrap(),
        GenerationCleanupOutcome::Unbound,
    )
    .await
    .unwrap();
    let completed = cleanup_outbox::Entity::find_by_id(claim.record.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, CLEANUP_STATUS_COMPLETED);
    assert_eq!(completed.outcome.as_deref(), Some("unbound"));
}
