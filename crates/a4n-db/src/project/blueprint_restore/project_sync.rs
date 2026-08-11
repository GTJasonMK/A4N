use super::*;

pub(super) async fn sync_project_after_blueprint_restore_record<C>(
    db: &C,
    project_id: i64,
    blueprint: &novel_blueprint::Model,
    data: &Value,
    now: NaiveDateTime,
) -> Result<Option<BlueprintRestoreStatusChange>, BlueprintRestoreError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|error| BlueprintRestoreError::database("读取恢复目标项目失败", error))?
        .ok_or(BlueprintRestoreError::ProjectNotFound { project_id })?;
    let previous_status = parse_project_status(&project.status).map_err(|_| {
        BlueprintRestoreError::InvalidProjectStatus {
            project_id,
            status: project.status.clone(),
        }
    })?;
    let next_status = if previous_status.ordinal() < ProjectStatus::BlueprintReady.ordinal() {
        ProjectStatus::BlueprintReady
    } else {
        previous_status
    };
    let total_chapters = blueprint.total_chapters.ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot("蓝图版本缺少 total_chapters，无法同步项目章节目标")
    })?;

    let mut active: novel_project::ActiveModel = project.into();
    active.title = Set(blueprint.title.clone());
    active.status = Set(next_status.as_str().to_string());
    active.target_chapters = Set(total_chapters);
    active.total_chapters = Set(blueprint.total_chapters);
    active.chapters_per_part = Set(blueprint.chapters_per_part);
    active.raw_blueprint = Set(Some(
        serde_json::to_string_pretty(data)
            .map_err(|source| BlueprintRestoreError::Serialization { source })?,
    ));
    active.updated_at = Set(now);
    active
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
            reason: Some("蓝图版本恢复".to_string()),
            changed_by: Some("system".to_string()),
            changed_at: now,
        },
    )
    .await
    .map_err(|source| BlueprintRestoreError::StatusWrite { source })?;
    Ok(Some(BlueprintRestoreStatusChange {
        from: previous_status,
        to: next_status,
        changed_at: now,
    }))
}
