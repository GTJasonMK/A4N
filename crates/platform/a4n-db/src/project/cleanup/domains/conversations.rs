use super::super::{ProjectCleanupDbError, ProjectConversationCleanupResult};
use crate::project::validate_positive_i64;
use a4n_db_models::models::user::{conversation, message};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn delete_project_conversations<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectConversationCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let conversation_ids = conversation::Entity::find()
        .select_only()
        .column(conversation::Column::Id)
        .filter(conversation::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let deleted_messages = if conversation_ids.is_empty() {
        0
    } else {
        message::Entity::delete_many()
            .filter(message::Column::ConversationId.is_in(conversation_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_conversations = conversation::Entity::delete_many()
        .filter(conversation::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectConversationCleanupResult {
        deleted_conversations,
        deleted_messages,
    })
}
