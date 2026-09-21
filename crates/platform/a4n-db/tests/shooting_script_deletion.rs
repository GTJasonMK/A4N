use a4n_db::shooting_script::{
    DeleteStoredChapterShootingScriptInput, delete_chapter_shooting_script_and_invalidate,
};
use a4n_db_models::models::{chapter::chapter_shooting_script, task::cleanup_outbox};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set};

#[tokio::test]
async fn deleting_script_removes_record_and_queues_downstream_reset() {
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
        contract_version: Set("chapter_shooting_script_v1".into()),
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
    let reset = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::DedupeKey.eq(format!(
            "shooting-script-delete:{}:revision:2:project:{}",
            record.id, downstream.id
        )))
        .one(&db)
        .await
        .unwrap()
        .expect("delete should queue a downstream reset");
    assert_eq!(reset.project_id, Some(downstream.id));
}
