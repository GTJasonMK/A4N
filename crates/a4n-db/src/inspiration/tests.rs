use super::*;
use a4n_db_models::models::{
    chapter::chapter,
    user::{conversation, message},
};
use a4n_domain::inspiration::InspirationExchangeInsertInput;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn history_requires_existing_project_without_creating_conversation() {
    let db = setup_db().await;

    let error = inspiration_messages(&db, 999_999, 80)
        .await
        .expect_err("missing project history must be rejected");

    assert!(matches!(
        error,
        InspirationDbError::NotFound(message) if message == "项目不存在: 999999"
    ));
    assert_eq!(conversation::Entity::find().count(&db).await.unwrap(), 0);
    assert_eq!(message::Entity::find().count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn empty_history_does_not_create_conversation() {
    let db = setup_db().await;
    let project = text_project(&db, "空灵感历史").await;

    assert!(
        inspiration_messages(&db, project.id, 80)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(conversation::Entity::find().count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn insert_exchange_persists_user_then_assistant_and_updates_count() {
    let db = setup_db().await;
    let project = text_project(&db, "灵感双消息").await;

    ensure_inspiration_conversation(&db, project.id)
        .await
        .unwrap();
    let tx = db.begin().await.unwrap();
    insert_inspiration_exchange_in_transaction(
        &tx,
        InspirationExchangeInsertInput {
            project_id: project.id,
            user_input: "继续深化主角".to_string(),
            assistant_output: "可以从时间循环切入。".to_string(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let conversation = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .expect("active inspiration conversation must exist");
    assert_eq!(conversation.message_count, 2);
    let messages = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .order_by_asc(message::Column::CreatedAt)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content, "继续深化主角");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[1].content, "可以从时间循环切入。");
}

#[tokio::test]
async fn context_rows_keep_recent_chapter_filter_semantics_with_projection() {
    let db = setup_db().await;
    let project = text_project(&db, "灵感章节上下文").await;
    let now = chrono::Utc::now().naive_utc();

    for (chapter_number, content, summary) in [
        (1, "\u{3000}\t", None),
        (2, "正文", None),
        (3, " ", Some("摘要")),
    ] {
        chapter::Entity::insert(chapter::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            content: Set(content.to_string()),
            word_count: Set(content.chars().count() as i32),
            status: Set(chapter::STATUS_DRAFT.to_string()),
            summary: Set(summary.map(str::to_string)),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .exec(&db)
        .await
        .unwrap();
    }

    let rows = inspiration_project_context_rows(&db, project.id)
        .await
        .unwrap();
    assert_eq!(rows.recent_chapters.len(), 3);
    assert_eq!(rows.recent_chapters[0].chapter_number, 1);
    assert!(!rows.recent_chapters[0].has_content);
    assert_eq!(rows.recent_chapters[1].chapter_number, 2);
    assert!(rows.recent_chapters[1].has_content);
    assert_eq!(rows.recent_chapters[2].chapter_number, 3);
    assert!(!rows.recent_chapters[2].has_content);
    assert_eq!(rows.recent_chapters[2].summary.as_deref(), Some("摘要"));
}
