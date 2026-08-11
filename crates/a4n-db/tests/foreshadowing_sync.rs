use a4n_core::models::{chapter::chapter, character::foreshadowing_index};
use a4n_db::project::{
    ChapterSourceSnapshot, CreateProjectRecordInput, ForeshadowingAnalysisDraft,
    ForeshadowingDbError, ForeshadowingRecordInput, PROJECT_TYPE_NOVEL_LONG,
    cleanup_foreshadowing_for_chapter, create_foreshadowing_record, create_project_record,
    delete_foreshadowing_record, sync_chapter_analysis_foreshadowing, update_foreshadowing_record,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn create_project(db: &DatabaseConnection) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "章节伏笔同步测试".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

async fn insert_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(format!("第{chapter_number}章"))),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("chapter must be inserted")
}

async fn insert_foreshadowing(
    db: &DatabaseConnection,
    project_id: i64,
    description: &str,
    setup_chapter: i32,
    payoff_chapter: Option<i32>,
    status: &str,
) -> foreshadowing_index::Model {
    let now = Utc::now().naive_utc();
    foreshadowing_index::ActiveModel {
        project_id: Set(project_id),
        description: Set(description.to_string()),
        setup_chapter: Set(setup_chapter),
        payoff_chapter: Set(payoff_chapter),
        status: Set(status.to_string()),
        related_characters: Set(None),
        importance: Set(3),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("foreshadowing must be inserted")
}

#[tokio::test]
async fn sync_resolves_idempotently_and_preserves_when_reanalysis_omits_it() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = insert_chapter(&db, project_id, 8).await;
    let source = ChapterSourceSnapshot::from(&chapter);
    let planted = insert_foreshadowing(&db, project_id, "门后的脚步声", 2, None, "pending").await;

    let first =
        sync_chapter_analysis_foreshadowing(&db, source.clone(), Vec::new(), vec![planted.id])
            .await
            .expect("first resolution must succeed");
    let repeated =
        sync_chapter_analysis_foreshadowing(&db, source.clone(), Vec::new(), vec![planted.id])
            .await
            .expect("same resolution must be idempotent");
    assert_eq!(
        first
            .resolved
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        [planted.id]
    );
    assert_eq!(
        repeated
            .existing
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        [planted.id]
    );
    let resolved = foreshadowing_index::Entity::find_by_id(planted.id)
        .one(&db)
        .await
        .expect("lookup")
        .expect("foreshadowing");
    assert_eq!(resolved.status, "resolved");
    assert_eq!(resolved.payoff_chapter, Some(8));

    sync_chapter_analysis_foreshadowing(&db, source, Vec::new(), Vec::new())
        .await
        .expect("omitted resolution must remain unchanged");
    let preserved = foreshadowing_index::Entity::find_by_id(planted.id)
        .one(&db)
        .await
        .expect("lookup")
        .expect("foreshadowing");
    assert_eq!(preserved.status, "resolved");
    assert_eq!(preserved.payoff_chapter, Some(8));
}

#[tokio::test]
async fn sync_reports_created_updated_and_existing_setups() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = insert_chapter(&db, project_id, 8).await;
    insert_foreshadowing(&db, project_id, "保持不变", 8, None, "pending").await;
    insert_foreshadowing(&db, project_id, "需要更新", 8, None, "pending").await;

    let result = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&chapter),
        vec![
            ForeshadowingAnalysisDraft {
                description: "保持不变".to_string(),
                status: "pending".to_string(),
                related_characters: None,
                importance: 3,
            },
            ForeshadowingAnalysisDraft {
                description: "需要更新".to_string(),
                status: "pending".to_string(),
                related_characters: Some(json!(["主角"])),
                importance: 5,
            },
            ForeshadowingAnalysisDraft {
                description: "本章新增".to_string(),
                status: "pending".to_string(),
                related_characters: None,
                importance: 4,
            },
        ],
        Vec::new(),
    )
    .await
    .expect("setup sync must report its actions");

    assert_eq!(result.existing[0].description, "保持不变");
    assert_eq!(result.updated[0].description, "需要更新");
    assert_eq!(result.updated[0].importance, 3);
    assert_eq!(result.updated[0].related_characters, Some(json!(["主角"])));
    assert_eq!(result.created[0].description, "本章新增");
}

#[tokio::test]
async fn sync_preserves_setup_that_a_repeat_analysis_does_not_return() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = insert_chapter(&db, project_id, 8).await;
    let planted = insert_foreshadowing(&db, project_id, "门后的脚步声", 8, None, "pending").await;

    let result = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&chapter),
        Vec::new(),
        Vec::new(),
    )
    .await
    .expect("repeat analysis omission must be non-destructive");

    assert!(result.stale_ids.is_empty());
    assert!(
        foreshadowing_index::Entity::find_by_id(planted.id)
            .one(&db)
            .await
            .expect("lookup")
            .is_some()
    );
}

#[tokio::test]
async fn invalid_resolution_rolls_back_current_chapter_reopen() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = insert_chapter(&db, project_id, 8).await;
    let resolved = insert_foreshadowing(&db, project_id, "旧友身份", 2, Some(8), "resolved").await;

    let error = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&chapter),
        Vec::new(),
        vec![i64::MAX],
    )
    .await
    .expect_err("unknown id must fail");
    assert!(matches!(
        error,
        ForeshadowingDbError::InvalidResolutionTarget { .. }
    ));

    let unchanged = foreshadowing_index::Entity::find_by_id(resolved.id)
        .one(&db)
        .await
        .expect("lookup")
        .expect("foreshadowing");
    assert_eq!(unchanged.status, "resolved");
    assert_eq!(unchanged.payoff_chapter, Some(8));
}

#[tokio::test]
async fn stale_chapter_source_rejects_all_foreshadowing_writes() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = insert_chapter(&db, project_id, 8).await;
    let source = ChapterSourceSnapshot::from(&chapter);
    let pending = insert_foreshadowing(&db, project_id, "门后的脚步声", 2, None, "pending").await;
    let mut changed: chapter::ActiveModel = chapter.into();
    changed.content = Set("用户重写后的正文".to_string());
    changed.updated_at = Set(source.updated_at + Duration::seconds(1));
    changed
        .update(&db)
        .await
        .expect("chapter rewrite must succeed");

    let error = sync_chapter_analysis_foreshadowing(
        &db,
        source,
        vec![ForeshadowingAnalysisDraft {
            description: "旧模型识别的新伏笔".to_string(),
            status: "pending".to_string(),
            related_characters: None,
            importance: 3,
        }],
        vec![pending.id],
    )
    .await
    .expect_err("stale chapter analysis must not write foreshadowing");
    assert!(matches!(
        error,
        ForeshadowingDbError::InvalidState { current, .. }
            if current == "chapter_source_changed"
    ));

    let unchanged = foreshadowing_index::Entity::find_by_id(pending.id)
        .one(&db)
        .await
        .expect("lookup")
        .expect("pending foreshadowing must remain");
    assert_eq!(unchanged.status, "pending");
    assert_eq!(unchanged.payoff_chapter, None);
    assert_eq!(
        foreshadowing_index::Entity::find()
            .all(&db)
            .await
            .expect("foreshadowing rows must load")
            .len(),
        1
    );
}

#[tokio::test]
async fn cleanup_preserves_formal_setups_and_reopens_current_payoffs() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let current_setup =
        insert_foreshadowing(&db, project_id, "本章新伏笔", 8, None, "pending").await;
    let current_payoff =
        insert_foreshadowing(&db, project_id, "本章回收旧伏笔", 2, Some(8), "resolved").await;
    let unrelated = insert_foreshadowing(&db, project_id, "其他章节伏笔", 3, None, "pending").await;

    let tx = db.begin().await.expect("transaction");
    let result = cleanup_foreshadowing_for_chapter(&tx, project_id, 8)
        .await
        .expect("cleanup");
    tx.commit().await.expect("commit");
    assert!(result.stale_ids.is_empty());
    assert_eq!(result.changed_ids, [current_payoff.id]);
    assert!(
        foreshadowing_index::Entity::find_by_id(current_setup.id)
            .one(&db)
            .await
            .expect("lookup")
            .is_some()
    );
    let reopened = foreshadowing_index::Entity::find_by_id(current_payoff.id)
        .one(&db)
        .await
        .expect("lookup")
        .expect("payoff");
    assert_eq!(reopened.status, "pending");
    assert_eq!(reopened.payoff_chapter, None);
    assert!(
        foreshadowing_index::Entity::find_by_id(unrelated.id)
            .one(&db)
            .await
            .expect("lookup")
            .is_some()
    );
}

#[tokio::test]
async fn manual_foreshadowing_crud_rejects_stale_forms() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let created = create_foreshadowing_record(
        &db,
        project_id,
        ForeshadowingRecordInput {
            description: "  门后的脚步声  ".to_string(),
            setup_chapter: 2,
            payoff_chapter: None,
            status: "pending".to_string(),
            related_characters: vec![" 主角 ".to_string(), "主角".to_string()],
            importance: 4,
        },
    )
    .await
    .expect("manual record must be created");
    assert_eq!(created.description, "门后的脚步声");
    assert_eq!(created.related_characters, Some(json!(["主角"])));

    let stale_version = created.updated_at.to_string();
    let updated = update_foreshadowing_record(
        &db,
        project_id,
        created.id,
        &stale_version,
        ForeshadowingRecordInput {
            description: created.description.clone(),
            setup_chapter: 2,
            payoff_chapter: Some(8),
            status: "resolved".to_string(),
            related_characters: vec!["主角".to_string(), "旧友".to_string()],
            importance: 5,
        },
    )
    .await
    .expect("current form must update");

    let error = delete_foreshadowing_record(&db, project_id, created.id, &stale_version)
        .await
        .expect_err("stale form must not delete newer data");
    assert!(matches!(
        error,
        ForeshadowingDbError::InvalidState { current, .. }
            if current == "foreshadowing_source_changed"
    ));
    delete_foreshadowing_record(&db, project_id, updated.id, &updated.updated_at.to_string())
        .await
        .expect("current form must delete");
    assert!(
        foreshadowing_index::Entity::find_by_id(created.id)
            .one(&db)
            .await
            .expect("lookup")
            .is_none()
    );
}
