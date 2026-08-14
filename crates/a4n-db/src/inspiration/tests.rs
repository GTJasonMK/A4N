use super::*;
use a4n_core::models::user::{conversation, message};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
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
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
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
