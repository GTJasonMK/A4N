use sea_orm::{DatabaseConnection, DbErr, TransactionTrait};

use super::cleanup::delete_media_workflow_v2_records;
use super::contracts::MediaWorkflowV2ResetOutcome;
use super::rules::{
    validate_media_workflow_reset_project_id, validate_media_workflow_reset_project_kind,
};
use super::scope::load_media_workflow_v2_reset_scope;

pub async fn reset_media_workflows_to_v2(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<MediaWorkflowV2ResetOutcome, DbErr> {
    validate_media_workflow_reset_project_id(project_id)?;
    let tx = db.begin().await?;
    let project = crate::project::project_by_id_for_update(&tx, project_id)
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?
        .ok_or_else(|| DbErr::Custom(format!("项目不存在: {project_id}")))?;
    validate_media_workflow_reset_project_kind(&project.project_kind)?;

    let scope = load_media_workflow_v2_reset_scope(&tx, project_id).await?;
    delete_media_workflow_v2_records(&tx, project_id, &scope).await?;
    let media_paths =
        crate::media::filter_unreferenced_media_paths(&tx, &scope.media_paths).await?;
    tx.commit().await?;
    Ok(MediaWorkflowV2ResetOutcome { media_paths })
}
