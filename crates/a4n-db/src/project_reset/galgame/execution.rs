use a4n_core::models::project::novel_project;
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};

use super::super::types::{ProjectGeneratedDataResetOutcome, ProjectResetDbError};
use super::result::{GalgameResetOutcomeInput, galgame_reset_outcome};
use crate::galgame::delete_galgame_script_records_for_project;
use crate::project::{self as project_db, ProjectStatusHistoryInput};
use crate::project_delete::{GeneratedImageDeleteScope, delete_chapter_generation_outputs};

pub(in crate::project_reset) async fn reset_galgame_generated_data_in_tx(
    tx: DatabaseTransaction,
    project: novel_project::Model,
    candidate_media_paths: Vec<String>,
) -> Result<ProjectGeneratedDataResetOutcome, ProjectResetDbError> {
    let project_id = project.id;
    let previous_status = StateValidator::parse_status(&project.status).map_err(|_| {
        ProjectResetDbError::InvalidState {
            current: project.status.clone(),
            operation: "重置 Galgame 项目生成数据".to_string(),
        }
    })?;
    let now = Utc::now().naive_utc();

    let optimization_cleanup =
        project_db::delete_project_optimization_tree(&tx, project_id).await?;
    let chapter_generation_cleanup =
        delete_chapter_generation_outputs(&tx, project_id, GeneratedImageDeleteScope::Project)
            .await?;
    let generation_media_cleanup =
        project_db::delete_project_generation_media_graph(&tx, project_id).await?;
    let galgame_cleanup = delete_galgame_script_records_for_project(&tx, project_id).await?;
    let conversation_cleanup = project_db::delete_project_conversations(&tx, project_id).await?;
    let task_cleanup = project_db::delete_project_task_queue_records(&tx, project_id).await?;

    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(ProjectStatus::Draft.as_str().to_string());
    active.current_chapter = Set(0);
    active.total_chapters = Set(None);
    active.import_analysis_progress = Set(None);
    active.updated_at = Set(now);
    let updated = active.update(&tx).await?;

    project_db::insert_project_status_history(
        &tx,
        ProjectStatusHistoryInput {
            project_id,
            from_status: previous_status.as_str().to_string(),
            to_status: ProjectStatus::Draft.as_str().to_string(),
            reason: Some("Galgame 项目生成数据重置".to_string()),
            changed_by: Some("system".to_string()),
            changed_at: now,
        },
    )
    .await?;

    let media_paths = crate::media::schedule_unreferenced_media_cleanup(
        &tx,
        project_id,
        &candidate_media_paths,
        format!("project-generated-data-reset:{project_id}"),
    )
    .await?;
    // gate 保持 resetting；Host 清理 RAG 成功后再完成，崩溃时由同一 gate 接续。
    tx.commit().await?;

    Ok(galgame_reset_outcome(GalgameResetOutcomeInput {
        project: updated,
        previous_status,
        optimization_cleanup,
        chapter_generation_cleanup,
        generation_media_cleanup,
        galgame_cleanup,
        conversation_cleanup,
        task_cleanup,
        media_paths,
    }))
}
