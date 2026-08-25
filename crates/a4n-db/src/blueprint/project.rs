use a4n_core::models::project::novel_project;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, SyncProjectAfterBlueprintInput};

/// 蓝图生成完成后同步项目标题、状态、章节规划与 raw-blueprint。
pub async fn sync_project_after_blueprint_record<C>(
    db: &C,
    input: SyncProjectAfterBlueprintInput,
) -> Result<novel_project::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    novel_project::ActiveModel {
        id: Set(input.project_id),
        title: Set(input.title),
        status: Set(input.status.as_str().to_string()),
        target_chapters: Set(input.target_chapters),
        total_chapters: Set(input.total_chapters),
        chapters_per_part: Set(input.chapters_per_part),
        raw_blueprint: Set(Some(input.raw_blueprint)),
        updated_at: Set(input.now),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(|error| match error {
        sea_orm::DbErr::RecordNotFound(_) => {
            BlueprintDbError::NotFound(format!("项目不存在: {}", input.project_id))
        }
        other => BlueprintDbError::Database(other),
    })
}
