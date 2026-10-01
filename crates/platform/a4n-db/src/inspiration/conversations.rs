use super::contracts::InspirationDbError;
use super::project::ensure_inspiration_project_exists;
use a4n_db_models::models::user::{conversation, message};
use a4n_domain::inspiration::{InspirationExchangeInsertInput, InspirationMessageRecord};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

const INSPIRATION_CONVERSATION_TYPE: &str = "inspiration";
const INSPIRATION_CONVERSATION_TITLE: &str = "灵感对话";
/// 界面历史列表上限；模型调用传 None，保留此前仍有效的用户约束。
pub const INSPIRATION_HISTORY_LIMIT: u64 = 80;

async fn active_inspiration_conversation<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<conversation::Model>, InspirationDbError>
where
    C: sea_orm::ConnectionTrait,
{
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

pub async fn ensure_inspiration_conversation(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<(), InspirationDbError> {
    ensure_inspiration_project_exists(db, project_id).await?;
    if active_inspiration_conversation(db, project_id)
        .await?
        .is_none()
    {
        create_inspiration_conversation(db, project_id).await?;
    }
    Ok(())
}

pub async fn inspiration_messages(
    db: &DatabaseConnection,
    project_id: i64,
    limit: Option<u64>,
) -> Result<Vec<InspirationMessageRecord>, InspirationDbError> {
    ensure_inspiration_project_exists(db, project_id).await?;
    inspiration_messages_for_connection(db, project_id, limit).await
}

pub async fn inspiration_messages_for_connection<C>(
    db: &C,
    project_id: i64,
    limit: Option<u64>,
) -> Result<Vec<InspirationMessageRecord>, InspirationDbError>
where
    C: sea_orm::ConnectionTrait,
{
    let Some(conversation) = active_inspiration_conversation(db, project_id).await? else {
        return Ok(Vec::new());
    };
    let mut rows = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .order_by_desc(message::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(InspirationDbError::Database)?;
    rows.reverse();
    Ok(rows
        .into_iter()
        .map(|message| InspirationMessageRecord {
            id: message.id,
            role: message.role,
            content: message.content,
            created_at: message.created_at,
        })
        .collect())
}

pub async fn insert_inspiration_exchange_in_transaction(
    tx: &DatabaseTransaction,
    input: InspirationExchangeInsertInput,
) -> Result<i64, InspirationDbError> {
    let conversation = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.eq(input.project_id))
        .filter(conversation::Column::ConversationType.eq(INSPIRATION_CONVERSATION_TYPE))
        .filter(conversation::Column::IsActive.eq(true))
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(InspirationDbError::Database)?
        .ok_or_else(|| {
            InspirationDbError::NotFound(format!("项目 {} 的灵感对话不存在", input.project_id))
        })?;
    let next_message_count = conversation.message_count.checked_add(2).ok_or_else(|| {
        InspirationDbError::InvalidArgument("对话消息数超出 i32 范围".to_string())
    })?;
    let now = Utc::now().naive_utc();
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
    .insert(tx)
    .await
    .map_err(InspirationDbError::Database)?;
    let assistant = message::ActiveModel {
        conversation_id: Set(conversation.id),
        role: Set("assistant".to_string()),
        content: Set(input.assistant_output),
        tool_calls: Set(None),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        chapter_number: Set(None),
        is_important: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(tx)
    .await
    .map_err(InspirationDbError::Database)?;

    let mut active: conversation::ActiveModel = conversation.into();
    active.message_count = Set(next_message_count);
    active.updated_at = Set(now);
    active
        .update(tx)
        .await
        .map_err(InspirationDbError::Database)?;
    Ok(assistant.id)
}
