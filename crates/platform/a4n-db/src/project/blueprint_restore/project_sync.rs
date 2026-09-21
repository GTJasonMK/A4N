use super::*;
use a4n_domain::project::ProjectStatusHistoryInput;
use sea_orm::QuerySelect;

pub(super) async fn sync_project_after_blueprint_restore_record<C>(
    db: &C,
    project_id: i64,
    blueprint: &novel_blueprint::Model,
    data: &Value,
    now: NaiveDateTime,
) -> Result<Option<crate::blueprint::BlueprintMutationStatusChange>, BlueprintRestoreError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Status)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(|error| BlueprintRestoreError::database("读取恢复目标项目失败", error))?
        .ok_or(BlueprintRestoreError::ProjectNotFound { project_id })?;
    let previous_status = ProjectStatus::parse_storage(&project).map_err(|_| {
        BlueprintRestoreError::InvalidProjectStatus {
            project_id,
            status: project.clone(),
        }
    })?;
    let next_status =
        a4n_domain::project::project_status_after_blueprint_generation(previous_status);
    let total_chapters = blueprint.total_chapters.ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot("蓝图版本缺少 total_chapters，无法同步项目章节目标")
    })?;

    novel_project::ActiveModel {
        id: Set(project_id),
        title: Set(blueprint.title.clone()),
        status: Set(next_status.as_str().to_string()),
        target_chapters: Set(total_chapters),
        total_chapters: Set(blueprint.total_chapters),
        chapters_per_part: Set(blueprint.chapters_per_part),
        raw_blueprint: Set(Some(
            serde_json::to_string_pretty(data)
                .map_err(|source| BlueprintRestoreError::Serialization { source })?,
        )),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(|error| BlueprintRestoreError::database("同步恢复后的项目失败", error))?;

    if previous_status == next_status {
        return Ok(None);
    }

    insert_project_status_history(
        db,
        ProjectStatusHistoryInput {
            project_id,
            from_status: previous_status,
            to_status: next_status,
            reason: "蓝图版本恢复".to_string(),
            changed_by: "system".to_string(),
            changed_at: now,
        },
    )
    .await
    .map_err(|source| BlueprintRestoreError::StatusWrite { source })?;
    Ok(Some(crate::blueprint::BlueprintMutationStatusChange {
        from: previous_status,
        to: next_status,
        changed_at: now,
    }))
}
