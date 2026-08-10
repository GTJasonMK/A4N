use super::{
    ChapterContentEditError, ChapterContentImportError, EditChapterContentInput,
    GeneratedChapterContentRecordInput, ImportChapterContentInput,
    finish_generated_chapter_content_record_in_transaction, prepare_import_chapter_content,
    save_chapter_content_edit_record, upsert_import_chapter_content,
};
use crate::project::{CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG};
use a4n_core::models::chapter::{chapter, chapter_outline};
use a4n_core::models::project::novel_project;
use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set,
    TransactionTrait,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn create_project(db: &DatabaseConnection) -> i64 {
    crate::project::create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "章节正文同步测试".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

fn test_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 8)
        .expect("valid test date")
        .and_hms_opt(12, 0, 0)
        .expect("valid test time")
}

fn import_input(project_id: i64, chapter_number: i32, manual: bool) -> ImportChapterContentInput {
    let content = format!("第{chapter_number}章正文");
    ImportChapterContentInput {
        project_id,
        chapter_number,
        content: content.clone(),
        normalized_title: Some(format!("第{chapter_number}章")),
        manual,
        word_count: content.chars().count() as i32,
        now: test_time(),
    }
}

async fn import_chapter_content_record(
    db: &DatabaseConnection,
    input: ImportChapterContentInput,
) -> Result<chapter::Model, ChapterContentImportError> {
    let tx = db.begin().await?;
    let existing = prepare_import_chapter_content(&tx, &input).await?;
    let chapter = upsert_import_chapter_content(&tx, existing, input).await?;
    tx.commit().await?;
    Ok(chapter)
}

async fn insert_outline(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter_outline::Model {
    let now = test_time();
    chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some("用户章纲标题".to_string())),
        summary: Set(Some("用户章纲摘要".to_string())),
        beats: Set(Some(serde_json::json!(["用户节拍"]))),
        key_scenes: Set(Some(serde_json::json!(["用户场景"]))),
        pacing_score: Set(Some(0.75)),
        foreshadowing_setup: Set(Some(serde_json::json!(["用户伏笔"]))),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(Some(serde_json::json!(["用户角色"]))),
        word_count_target: Set(Some(1234)),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("test outline must be created")
}

#[tokio::test]
async fn manual_import_creates_chapter_two_without_previous_and_marks_draft() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;

    let chapter = import_chapter_content_record(&db, import_input(project_id, 2, true))
        .await
        .expect("manual chapter creation must not require chapter one");

    assert_eq!(chapter.chapter_number, 2);
    assert_eq!(chapter.status, chapter::STATUS_DRAFT);
    assert!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.eq(1))
            .one(&db)
            .await
            .expect("previous chapter lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn non_manual_import_creates_imported_chapter() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;

    let chapter = import_chapter_content_record(&db, import_input(project_id, 2, false))
        .await
        .expect("external chapter import must succeed");

    assert_eq!(chapter.status, chapter::STATUS_IMPORTED);
}

#[tokio::test]
async fn manual_and_external_writes_update_status_without_requiring_previous_chapter() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;

    let imported = import_chapter_content_record(&db, import_input(project_id, 7, false))
        .await
        .expect("external import must create a non-linear chapter");
    assert_eq!(imported.status, chapter::STATUS_IMPORTED);

    let manual = import_chapter_content_record(&db, import_input(project_id, 7, true))
        .await
        .expect("manual save must reuse the same chapter write path");
    assert_eq!(manual.status, chapter::STATUS_DRAFT);

    let external_again = import_chapter_content_record(&db, import_input(project_id, 7, false))
        .await
        .expect("external re-import must remain supported");
    assert_eq!(external_again.status, chapter::STATUS_IMPORTED);
}

#[tokio::test]
async fn import_preserves_existing_outline_and_does_not_create_missing_outline() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let existing_outline = insert_outline(&db, project_id, 4).await;

    let imported = import_chapter_content_record(&db, import_input(project_id, 4, false))
        .await
        .expect("chapter import must succeed with an existing outline");
    assert_eq!(imported.outline_id, Some(existing_outline.id));
    assert_eq!(
        chapter::Entity::find_by_id(imported.id)
            .one(&db)
            .await
            .expect("chapter lookup must succeed")
            .expect("imported chapter must remain")
            .outline_id,
        Some(existing_outline.id)
    );

    let preserved_outline = chapter_outline::Entity::find_by_id(existing_outline.id)
        .one(&db)
        .await
        .expect("outline lookup must succeed")
        .expect("existing outline must remain");
    assert_eq!(preserved_outline, existing_outline);

    import_chapter_content_record(&db, import_input(project_id, 5, false))
        .await
        .expect("chapter import without an outline must succeed");
    assert!(
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(5))
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn content_write_clears_stale_outline_link_when_no_matching_outline_exists() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let other_project_id = create_project(&db).await;
    let stale_outline = insert_outline(&db, other_project_id, 6).await;

    let chapter = chapter::ActiveModel {
        id: NotSet,
        project_id: Set(project_id),
        outline_id: Set(Some(stale_outline.id)),
        chapter_number: Set(6),
        title: Set(Some("第6章".to_string())),
        content: Set("旧正文".to_string()),
        word_count: Set(3),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(test_time()),
        updated_at: Set(test_time()),
    }
    .insert(&db)
    .await
    .expect("chapter with stale link must be inserted");

    let updated = import_chapter_content_record(&db, import_input(project_id, 6, true))
        .await
        .expect("manual content write must repair stale link");

    assert_eq!(updated.id, chapter.id);
    assert_eq!(updated.outline_id, None);
    assert_eq!(
        chapter::Entity::find_by_id(chapter.id)
            .one(&db)
            .await
            .expect("chapter lookup must succeed")
            .expect("chapter must remain")
            .outline_id,
        None
    );
}

#[tokio::test]
async fn generated_content_rejects_an_outline_created_after_preparation() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let outline = insert_outline(&db, project_id, 8).await;

    let error = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id,
            chapter_number: 8,
            existing_chapter_id: None,
            // Simulates brief-only preparation while a user creates an outline
            // before the LLM result is committed.
            outline_id: None,
            outline_updated_at: None,
            title: "第 8 章".to_string(),
            summary: None,
            content: "按简报写成的正文".to_string(),
            word_count: 8,
            now: test_time(),
        },
    )
    .await
    .expect_err("new outline must invalidate brief-only generation");

    assert!(error.to_string().contains("章纲已变化"));
    assert!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.eq(8))
            .one(&db)
            .await
            .expect("chapter lookup must succeed")
            .is_none()
    );
    assert!(
        chapter_outline::Entity::find_by_id(outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_some()
    );
}

#[tokio::test]
async fn chapter_content_writes_reject_a_resetting_project() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let chapter = import_chapter_content_record(&db, import_input(project_id, 1, true))
        .await
        .expect("initial chapter must be created while project is active");

    let project = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .expect("project lookup must succeed")
        .expect("project must exist");
    let mut active: novel_project::ActiveModel = project.into();
    active.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
    active
        .update(&db)
        .await
        .expect("test must mark project as resetting");

    let edit_error = save_chapter_content_edit_record(
        &db,
        EditChapterContentInput {
            chapter_id: chapter.id,
            content: "重置期间的新正文".to_string(),
            title: None,
        },
    )
    .await
    .expect_err("manual edit must be rejected while resetting");
    assert!(matches!(
        edit_error,
        ChapterContentEditError::InvalidState { current, .. }
            if current == crate::generation::PROJECT_GATE_RESETTING
    ));

    let import_error = import_chapter_content_record(&db, import_input(project_id, 2, false))
        .await
        .expect_err("external import must be rejected while resetting");
    assert!(matches!(
        import_error,
        ChapterContentImportError::InvalidState { current, .. }
            if current == crate::generation::PROJECT_GATE_RESETTING
    ));

    let generation_error = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id,
            chapter_number: 3,
            existing_chapter_id: None,
            outline_id: None,
            outline_updated_at: None,
            title: "第 3 章".to_string(),
            summary: None,
            content: "重置期间生成的正文".to_string(),
            word_count: 9,
            now: test_time(),
        },
    )
    .await
    .expect_err("generated content must be rejected while resetting");
    assert!(matches!(
        generation_error,
        super::ChapterGenerationLifecycleError::InvalidState { current, .. }
            if current == crate::generation::PROJECT_GATE_RESETTING
    ));

    let stored = chapter::Entity::find_by_id(chapter.id)
        .one(&db)
        .await
        .expect("chapter lookup must succeed")
        .expect("original chapter must remain");
    assert_eq!(stored.content, chapter.content);
    assert!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.is_in([2, 3]))
            .all(&db)
            .await
            .expect("chapter lookup must succeed")
            .is_empty()
    );
}
