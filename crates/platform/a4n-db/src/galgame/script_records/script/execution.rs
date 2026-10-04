use super::deletion::delete_galgame_script_records_for_project;
use super::persistence::persist_galgame_script_record;
use crate::galgame::{
    GalgameScriptRecordError, PersistGalgameScriptRecordInput,
    delete_galgame_target_generation_media_graph_record,
};
use a4n_db_models::models::{galgame::galgame_script, project::novel_project};
use sea_orm::{EntityTrait, QuerySelect};

pub async fn replace_galgame_script_for_project_record(
    db: &impl crate::DbExecutor,
    project_id: i64,
    input: PersistGalgameScriptRecordInput,
) -> Result<galgame_script::Model, GalgameScriptRecordError> {
    let tx = db.begin().await?;
    let expected_story_graph_revision = input.expected_story_graph_revision;
    let story_graph = input.story_graph.clone();
    let project = novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| GalgameScriptRecordError::InvalidArgument("游戏项目不存在".into()))?;
    if input.project_id != project_id
        || project.source_snapshot_id != Some(input.source_snapshot_id)
        || project.root_project_id != Some(input.root_project_id)
        || project.lifecycle_gate != "active"
    {
        return Err(GalgameScriptRecordError::InvalidArgument(
            "游戏来源或项目状态已变化，请重新准备剧本".into(),
        ));
    }
    let reference = input.branch_plan.as_ref().ok_or_else(|| {
        GalgameScriptRecordError::InvalidArgument("生成游戏脚本必须记录实际采用的分支计划".into())
    })?;
    let branch = crate::media_authoring::authoring_document(&tx, project_id, reference.id)
        .await
        .map_err(authoring_error)?;
    if branch.reference() != *reference
        || branch.status != a4n_domain::media_authoring::AuthoringDocumentStatus::Adopted
        || branch.kind != a4n_domain::media_authoring::AuthoringDocumentKind::GalgameBranchPlan
        || branch.source_snapshot_id != input.source_snapshot_id
        || input
            .scenes
            .iter()
            .filter_map(|scene| scene.adaptation_plan_id)
            .any(|id| {
                !branch
                    .inputs
                    .dependencies
                    .iter()
                    .any(|parent| parent.id == id)
            })
    {
        return Err(GalgameScriptRecordError::InvalidArgument(
            "剧本依赖的分支或改编计划已变化".into(),
        ));
    }
    crate::media_authoring::validate_adopted_authoring_dependencies(
        &tx,
        project_id,
        &branch.inputs.dependencies,
    )
    .await
    .map_err(authoring_error)?;
    delete_galgame_target_generation_media_graph_record(&tx, project_id).await?;
    delete_galgame_script_records_for_project(&tx, project_id).await?;
    let script = persist_galgame_script_record(&tx, input).await?;
    crate::galgame::save_galgame_story_graph_revision_in_transaction(
        &tx,
        project_id,
        expected_story_graph_revision,
        "generated",
        story_graph,
    )
    .await?;
    tx.commit().await?;
    Ok(script)
}

fn authoring_error(
    error: crate::media_authoring::AuthoringDocumentError,
) -> GalgameScriptRecordError {
    match error {
        crate::media_authoring::AuthoringDocumentError::Database(error) => {
            GalgameScriptRecordError::Database(error)
        }
        error => GalgameScriptRecordError::InvalidArgument(error.to_string()),
    }
}
