use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::project::delete::ProjectDeleteDbError;
use crate::project::{self as project_db, PROJECT_KIND_TEXT};

use super::characters::delete_project_character_data;
use super::finalization::delete_project_final_records;
use super::media::delete_project_media_and_source_data;
use super::story::delete_project_story_data;

/// 删除文本根项目前检查派生数量；通过后执行级联删除。
pub async fn delete_project_cascade_record(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    let txn = db.begin().await?;
    let project = project_db::project_by_id_for_update(&txn, project_id)
        .await
        .map_err(ProjectDeleteDbError::from)?
        .ok_or(ProjectDeleteDbError::ProjectNotFound { project_id })?;
    crate::generation::require_project_lifecycle_gate(
        &txn,
        project_id,
        crate::generation::PROJECT_GATE_DELETING,
    )
    .await
    .map_err(|error| ProjectDeleteDbError::Database {
        source: sea_orm::DbErr::Custom(error.to_string()),
    })?;
    if project.project_kind == PROJECT_KIND_TEXT {
        let derived_count = project_db::derived_project_count(&txn, project_id).await?;
        if derived_count > 0 {
            return Err(ProjectDeleteDbError::DerivedProjectsExist {
                project_id,
                derived_count,
            });
        }
    }

    let media_paths = crate::project::project_generated_media_paths(&txn, project_id).await?;
    let dedupe_key = format!(
        "project_delete:{project_id}:{}",
        project.created_at.and_utc().timestamp_micros()
    );
    crate::cleanup_outbox::insert_project_deletion_cleanup_intent(
        &txn,
        dedupe_key,
        crate::cleanup_outbox::ProjectDeletionCleanupPayload::new(project_id, media_paths),
    )
    .await?;

    project_db::delete_project_chapter_adaptation_records(
        &txn,
        project_id,
        project_db::ProjectChapterAdaptationDeleteScope::Project,
    )
    .await?;
    delete_project_character_data(&txn, project_id).await?;
    delete_project_story_data(&txn, project_id).await?;
    delete_project_media_and_source_data(&txn, project_id).await?;
    delete_project_final_records(&txn, project_id).await?;
    txn.commit().await?;
    Ok(())
}
