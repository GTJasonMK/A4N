use a4n_core::models::character::foreshadowing_index;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

#[derive(Debug, Default)]
pub(super) struct ForeshadowingCleanupArtifacts {
    pub(super) stale_ids: Vec<i64>,
    pub(super) changed_ids: Vec<i64>,
}

pub(super) async fn cleanup_foreshadowing_for_chapter_in<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ForeshadowingCleanupArtifacts, String>
where
    C: ConnectionTrait,
{
    let mut artifacts = ForeshadowingCleanupArtifacts::default();

    let setup_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.eq(chapter_number))
        .all(db)
        .await
        .map_err(|err| format!("查询伏笔埋设索引失败: {err}"))?;
    for item in setup_foreshadowing {
        artifacts.stale_ids.push(item.id);
        foreshadowing_index::Entity::delete_by_id(item.id)
            .exec(db)
            .await
            .map_err(|err| format!("清理伏笔埋设索引失败: {err}"))?;
    }

    let revealed_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::PayoffChapter.eq(Some(chapter_number)))
        .all(db)
        .await
        .map_err(|err| format!("查询伏笔揭示索引失败: {err}"))?;
    for item in revealed_foreshadowing {
        artifacts.changed_ids.push(item.id);
        let mut active: foreshadowing_index::ActiveModel = item.into();
        active.payoff_chapter = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active
            .update(db)
            .await
            .map_err(|err| format!("重置伏笔揭示索引失败: {err}"))?;
    }

    Ok(artifacts)
}
