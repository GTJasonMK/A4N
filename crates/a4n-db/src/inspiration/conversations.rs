use super::contracts::{InspirationDbError, InspirationExchangeInsertInput};
use super::project::ensure_inspiration_project_exists;
use a4n_core::models::user::{conversation, message};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

const INSPIRATION_CONVERSATION_TYPE: &str = "inspiration";
const INSPIRATION_CONVERSATION_TITLE: &str = "灵感对话";

async fn active_inspiration_conversation(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<conversation::Model>, InspirationDbError> {
    conversation::Entity::find()
        .filter(conversation::Column::ProjectId.eq(project_id))
        .filter(conversation::Column::ConversationType.eq(INSPIRATION_CONVERSATION_TYPE))
        .filter(conversation::Column::IsActive.eq(true))
        .one(db)
        .await
        .map_err(InspirationDbError::Database)
}

async fn create_inspiration_conversation(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<conversation::Model, InspirationDbError> {
    let now = Utc::now().naive_utc();
    conversation::ActiveModel {
        project_id: Set(project_id),
        title: Set(Some(INSPIRATION_CONVERSATION_TITLE.to_string())),
        conversation_type: Set(INSPIRATION_CONVERSATION_TYPE.to_string()),
        summary: Set(None),
        extracted_facts: Set(None),
        message_count: Set(0),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(InspirationDbError::Database)
}

pub async fn get_or_create_inspiration_conversation(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<conversation::Model, InspirationDbError> {
    ensure_inspiration_project_exists(db, project_id).await?;
    if let Some(conversation) = active_inspiration_conversation(db, project_id).await? {
        return Ok(conversation);
    }
    create_inspiration_conversation(db, project_id).await
}

pub async fn inspiration_messages(
    db: &DatabaseConnection,
    project_id: i64,
    limit: u64,
) -> Result<Vec<message::Model>, InspirationDbError> {
    let conversation = get_or_create_inspiration_conversation(db, project_id).await?;
    let mut rows = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .order_by_desc(message::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(InspirationDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn insert_inspiration_exchange(
    db: &DatabaseConnection,
    input: InspirationExchangeInsertInput,
) -> Result<(), InspirationDbError> {
    let conversation = get_or_create_inspiration_conversation(db, input.project_id).await?;
    let next_message_count = conversation.message_count.checked_add(2).ok_or_else(|| {
        InspirationDbError::InvalidArgument("对话消息数超出 i32 范围".to_string())
    })?;
    let now = Utc::now().naive_utc();
    let tx = db.begin().await.map_err(InspirationDbError::Database)?;
    message::ActiveModel {
        conversation_id: Set(conversation.id),
        role: Set("user".to_string()),
        content: Set(input.user_input),
        tool_calls: Set(None),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        chapter_number: Set(None),
        is_important: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(InspirationDbError::Database)?;
    message::ActiveModel {
        conversation_id: Set(conversation.id),
        role: Set("assistant".to_string()),
        content: Set(input.assistant_output),
        tool_calls: Set(None),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        chapter_number: Set(None),
        is_important: Set(false),
        created_at: Set(now + Duration::milliseconds(1)),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(InspirationDbError::Database)?;

    let mut active: conversation::ActiveModel = conversation.into();
    active.message_count = Set(next_message_count);
    active.updated_at = Set(now);
    active
        .update(&tx)
        .await
        .map_err(InspirationDbError::Database)?;
    tx.commit().await.map_err(InspirationDbError::Database)
}
