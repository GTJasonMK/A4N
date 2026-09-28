use a4n_db_models::models::chapter::chapter_shooting_script;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};

use super::contracts::{DeleteStoredChapterShootingScriptInput, ShootingScriptDbError};

pub async fn delete_chapter_shooting_script_and_invalidate(
    db: &DatabaseConnection,
    input: DeleteStoredChapterShootingScriptInput,
) -> Result<(), ShootingScriptDbError> {
    if input.record_id <= 0 || input.expected_revision <= 0 {
        return Err(ShootingScriptDbError::InvalidArgument(
            "record_id 和 expected_revision 必须大于 0".to_string(),
        ));
    }

    let tx = db.begin().await?;
    let current = chapter_shooting_script::Entity::find_by_id(input.record_id)
        .one(&tx)
        .await?
        .ok_or_else(|| {
            ShootingScriptDbError::NotFound(format!("章节拍摄剧本不存在: {}", input.record_id))
        })?;
    if current.revision != input.expected_revision {
        return Err(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }

    let deleted = chapter_shooting_script::Entity::delete_many()
        .filter(chapter_shooting_script::Column::Id.eq(input.record_id))
        .filter(chapter_shooting_script::Column::Revision.eq(input.expected_revision))
        .exec(&tx)
        .await?;
    if deleted.rows_affected != 1 {
        return Err(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }

    tx.commit().await?;
    Ok(())
}
