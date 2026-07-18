use a4n_core::models::project::novel_project;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use super::{BlueprintDbError, SyncProjectAfterBlueprintInput};

/// 蓝图生成完成后同步项目标题、状态、章节规划与 raw-blueprint。
pub async fn sync_project_after_blueprint_record<C>(
    db: &C,
    input: SyncProjectAfterBlueprintInput,
) -> Result<novel_project::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(input.project_id)
        .one(db)
        .await?
        .ok_or_else(|| BlueprintDbError::NotFound(format!("项目不存在: {}", input.project_id)))?;
    let mut active: novel_project::ActiveModel = project.into();
    active.title = Set(input.title);
    active.status = Set(input.status);
    active.target_chapters = Set(input.target_chapters);
    active.total_chapters = Set(input.total_chapters);
    active.chapters_per_part = Set(input.chapters_per_part);
    active.raw_blueprint = Set(Some(input.raw_blueprint));
    active.updated_at = Set(input.now);
    Ok(active.update(db).await?)
}
