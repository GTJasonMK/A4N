use super::*;

/// 在单一事务内完成状态回退清理、状态写回、历史记录和派生项目标记。
pub async fn rollback_project_status_record(
    db: &DatabaseConnection,
    project_id: i64,
    expected_from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
) -> Result<ProjectStatusRollbackOutcome, ProjectResetDbError> {
    use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};

    let tx = db.begin().await?;
    let project = project_db::project_by_id_for_update(&tx, project_id)
        .await?
        .ok_or(ProjectResetDbError::ProjectNotFound { project_id })?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectResetDbError::UnsupportedProjectKind {
            project_kind: project.project_kind,
        });
    }
    let actual_from =
        parse_project_status(&project.status).map_err(|_| ProjectResetDbError::InvalidState {
            current: project.status.clone(),
            operation: "回退项目状态".to_string(),
        })?;
    if actual_from != expected_from {
        return Err(ProjectResetDbError::StatusChanged {
            expected: expected_from.as_str().to_string(),
            actual: actual_from.as_str().to_string(),
        });
    }
    if actual_from <= to {
        return Err(ProjectResetDbError::InvalidState {
            current: actual_from.as_str().to_string(),
            operation: format!("回退到状态 {}", to.as_str()),
        });
    }
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
    let updated = project_db::update_project_status_record_in(&tx, project, to, changed_at).await?;
    project_db::mark_derived_projects_root_updated_in_record(&tx, project_id, changed_at).await?;
    crate::media::schedule_unreferenced_media_cleanup(
        &tx,
        project_id,
        &candidate_media_paths,
        format!("project-status-rollback:{project_id}:{}", to.as_str()),
    )
    .await?;
    tx.commit().await?;
    Ok(ProjectStatusRollbackOutcome {
        project: updated,
        needs_rag_clear,
    })
}
