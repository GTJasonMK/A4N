use a4n_core::models::project::blueprint_version;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, InsertBlueprintVersionInput};

/// 插入蓝图版本快照记录。
pub async fn insert_blueprint_version_record<C>(
    db: &C,
    input: InsertBlueprintVersionInput,
) -> Result<blueprint_version::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    Ok(blueprint_version::ActiveModel {
        project_id: Set(input.project_id),
        version: Set(input.version),
        change_description: Set(input.change_description),
        blueprint_data: Set(input.blueprint_data),
        created_at: Set(input.now),
        created_by: Set(input.created_by),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
