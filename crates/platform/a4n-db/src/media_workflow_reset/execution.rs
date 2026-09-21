use sea_orm::{DatabaseConnection, DbErr, TransactionTrait};

use super::cleanup::delete_media_workflow_v2_records;
use super::rules::{
    validate_media_workflow_reset_project_id, validate_media_workflow_reset_project_kind,
};
use super::scope::load_media_workflow_v2_reset_scope;

pub async fn reset_media_workflows_to_v2(
    db: &DatabaseConnection,
    project_id: i64,
    cleanup_intent_id: i64,
) -> Result<(), DbErr> {
    validate_media_workflow_reset_project_id(project_id)?;
    if cleanup_intent_id <= 0 {
        return Err(DbErr::Custom(
            "media workflow reset cleanup_intent_id 必须是正整数".to_string(),
        ));
    }
    let tx = db.begin().await?;
    let project = crate::project::project_write_identity_for_update(&tx, project_id)
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?
        .ok_or_else(|| DbErr::Custom(format!("项目不存在: {project_id}")))?;
    validate_media_workflow_reset_project_kind(&project.project_kind)?;

    let scope = load_media_workflow_v2_reset_scope(&tx, project_id).await?;
    delete_media_workflow_v2_records(&tx, project_id, &scope).await?;
    crate::media::schedule_unreferenced_media_cleanup(
        &tx,
        project_id,
        &scope.media_paths,
        format!("media-workflow-v2-reset-files:{cleanup_intent_id}"),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
