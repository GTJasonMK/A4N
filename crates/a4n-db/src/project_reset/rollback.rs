use super::*;

/// 在单一事务内完成状态回退清理、状态写回、历史记录和派生项目标记。
pub async fn rollback_project_status_record(
    db: &DatabaseConnection,
    project_id: i64,
    from_status: &str,
    to_status: &str,
    changed_at: NaiveDateTime,
) -> Result<ProjectStatusRollbackOutcome, ProjectResetDbError> {
    use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};

    let expected_from = StateValidator::parse_status(from_status)
        .map_err(|error| ProjectResetDbError::InvalidArgument(error.to_string()))?;
    let to = StateValidator::parse_status(to_status)
        .map_err(|error| ProjectResetDbError::InvalidArgument(error.to_string()))?;
    let tx = db.begin().await?;
    let project = project_db::project_by_id_for_update(&tx, project_id)
        .await?
        .ok_or(ProjectResetDbError::ProjectNotFound { project_id })?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectResetDbError::UnsupportedProjectKind {
            project_kind: project.project_kind,
        });
    }
    let actual_from = StateValidator::parse_status(&project.status).map_err(|_| {
        ProjectResetDbError::InvalidState {
            current: project.status.clone(),
            operation: "回退项目状态".to_string(),
        }
    })?;
    if actual_from != expected_from {
        return Err(ProjectResetDbError::StatusChanged {
            expected: expected_from.as_str().to_string(),
            actual: actual_from.as_str().to_string(),
        });
    }
    StateValidator::validate_rollback(actual_from.as_str(), to.as_str()).map_err(|error| {
        match error {
            a4n_core::error::AppError::InvalidState { current, operation } => {
                ProjectResetDbError::InvalidState { current, operation }
            }
            a4n_core::error::AppError::InvalidArgument(message) => {
                ProjectResetDbError::InvalidArgument(message)
            }
            other => ProjectResetDbError::Internal(other.to_string()),
        }
    })?;
    let actions = rollback_actions(actual_from, to);
    let needs_rag_clear = actions
        .iter()
        .any(|action| matches!(action, CleanupAction::DeleteChapters));
    let mut candidate_media_paths = Vec::new();

    for action in actions {
        match action {
            CleanupAction::DeleteChapters => {
                let cleanup = delete_chapter_generation_outputs(
                    &tx,
                    project_id,
                    GeneratedImageDeleteScope::Chapters,
                )
                .await?;
                candidate_media_paths.extend(cleanup.candidate_media_paths);
                delete_project_profile_scene_data(&tx, project_id).await?;
                project_db::delete_project_task_queue_records(&tx, project_id).await?;
                chapter::Entity::delete_many()
                    .filter(chapter::Column::ProjectId.eq(project_id))
                    .exec(&tx)
                    .await?;
            }
            CleanupAction::DeleteChapterOutlines => {
                chapter_outline::Entity::delete_many()
                    .filter(chapter_outline::Column::ProjectId.eq(project_id))
                    .exec(&tx)
                    .await?;
            }
            CleanupAction::DeletePartOutlines => {
                part_outline::Entity::delete_many()
                    .filter(part_outline::Column::ProjectId.eq(project_id))
                    .exec(&tx)
                    .await?;
            }
            CleanupAction::DeleteBlueprintVersions => {
                project_db::delete_project_blueprints(&tx, project_id, true).await?;
            }
        }
    }
    let updated = project_db::update_project_status_record_in(
        &tx,
        project,
        project_db::UpdateProjectStatusRecordInput {
            to_status: to.as_str().to_string(),
            history_from_status: actual_from.as_str().to_string(),
            history_reason: "项目状态回退".to_string(),
            changed_by: Some("system".to_string()),
            changed_at,
            clear_draft_fields: to == ProjectStatus::Draft,
        },
    )
    .await?;
    project_db::mark_derived_projects_root_updated_in_record(&tx, project_id, changed_at).await?;
    let media_paths =
        crate::media::filter_unreferenced_media_paths(&tx, &candidate_media_paths).await?;
    tx.commit().await?;
    Ok(ProjectStatusRollbackOutcome {
        project: updated,
        media_paths,
        needs_rag_clear,
    })
}
