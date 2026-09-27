use a4n_db::shooting_script::{
    DeleteStoredChapterShootingScriptInput, ShootingScriptDbError,
    delete_chapter_shooting_script_and_invalidate,
};
use a4n_db_models::models::{
    ai::generated_image, chapter::chapter_shooting_script, project::novel_project,
    task::cleanup_outbox,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, QueryOrder, Set};

#[tokio::test]
async fn deleting_script_is_revision_fenced_and_preserves_downstream_state() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "拍摄剧本删除测试".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        a4n_domain::chapter::ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "正文".into(),
            title: None,
        },
        "shooting script deletion test",
    )
    .await
    .unwrap();
    let downstream = a4n_db::project::create_derived_project_record(
        &db,
        1,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "video".into(),
            title: "下游视频".into(),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let image = a4n_db::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: downstream.id,
            image_type: "imported".into(),
            file_path: format!("projects/{}/images/retained.png", downstream.id),
            prompt: "保留下游素材".into(),
            model: "test".into(),
            width: 1,
            height: 1,
            file_size: Some(1),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let downstream_before = novel_project::Entity::find_by_id(downstream.id)
        .one(&db)
        .await
        .unwrap();
    let cleanup_before = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(downstream.id))
        .order_by_asc(cleanup_outbox::Column::Id)
        .all(&db)
        .await
        .unwrap();
    let record = chapter_shooting_script::ActiveModel {
        id: NotSet,
        root_project_id: Set(root.id),
        source_chapter_id: Set(chapter.id),
        source_snapshot_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        source_revision_key: Set("chapter:1:revision:1".into()),
        source_content_sha256: Set("a".repeat(64)),
        visual_plan_id: Set(None),
        visual_plan_version: Set(None),
        visual_plan_sha256: Set(None),
        input_fingerprint_sha256: Set("b".repeat(64)),
        contract_version: Set("chapter_shooting_script_v2".into()),
        revision: Set(2),
        script_sha256: Set("c".repeat(64)),
        script_json: Set(serde_json::json!({})),
        workflow_run_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .unwrap();

    let stale = delete_chapter_shooting_script_and_invalidate(
        &db,
        DeleteStoredChapterShootingScriptInput {
            record_id: record.id,
            expected_revision: 1,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        stale,
        ShootingScriptDbError::RevisionConflict { .. }
    ));
    assert_eq!(
        chapter_shooting_script::Entity::find_by_id(record.id)
            .one(&db)
            .await
            .unwrap(),
        Some(record.clone())
    );

    delete_chapter_shooting_script_and_invalidate(
        &db,
        DeleteStoredChapterShootingScriptInput {
            record_id: record.id,
            expected_revision: 2,
        },
    )
    .await
    .unwrap();

    assert!(
        chapter_shooting_script::Entity::find_by_id(record.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let mut downstream_after = novel_project::Entity::find_by_id(downstream.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(downstream_after.source_sync_status, "root_updated");
    let downstream_before = downstream_before.unwrap();
    downstream_after.source_sync_status = downstream_before.source_sync_status.clone();
    downstream_after.updated_at = downstream_before.updated_at;
    assert_eq!(downstream_after, downstream_before);
    assert_eq!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap(),
        Some(image)
    );
    let cleanup_after = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(downstream.id))
        .order_by_asc(cleanup_outbox::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(cleanup_after, cleanup_before);
}
