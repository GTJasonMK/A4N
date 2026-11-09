use a4n_db_models::models::generation::{workflow_checkpoint, workflow_run};
use a4n_domain::manga::{MangaPipelineArtifactStage, manga_deleted_checkpoint_keys};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::Expr,
};

use super::{MangaPromptRecordError, manga_record_database};

pub(crate) async fn lock_manga_manual_write_scope<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<i64>, MangaPromptRecordError> {
    crate::generation::require_project_owner_lifecycle_active_for_update(db, project_id)
        .await
        .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::ChapterId.eq(chapter_id))
        .filter(
            Condition::any()
                .add(workflow_run::Column::WorkflowKind.eq("manga_pipeline"))
                .add(
                    Condition::all()
                        .add(workflow_run::Column::WorkflowKind.eq("image_generation"))
                        .add(workflow_run::Column::Mode.eq("manga_page")),
                ),
        )
        .order_by_asc(workflow_run::Column::Id)
        .lock_exclusive()
        .all(db)
        .await
        .map_err(manga_record_database("锁定漫画工作流失败"))?;
    if runs
        .iter()
        .any(|run| run.status == crate::workflow::WORKFLOW_STATUS_RUNNING)
    {
        return Err(MangaPromptRecordError::InvalidArgument(
            "漫画流水线正在运行，不能修改或删除产物".into(),
        ));
    }
    Ok(runs.into_iter().map(|run| run.id).collect())
}

pub(crate) async fn invalidate_deleted_manga_checkpoints<C: ConnectionTrait>(
    db: &C,
    run_ids: &[i64],
    chapter_id: i64,
    scope: Option<(MangaPipelineArtifactStage, Option<u32>)>,
) -> Result<(), MangaPromptRecordError> {
    if run_ids.is_empty() {
        return Ok(());
    }
    let mut deletion = workflow_checkpoint::Entity::delete_many()
        .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(run_ids.iter().copied()));
    if let Some((stage, page)) = scope {
        let keys = manga_deleted_checkpoint_keys(chapter_id, stage, page);
        let deletes_image = stage == MangaPipelineArtifactStage::Image
            || MangaPipelineArtifactStage::Image.depends_on(stage);
        let checkpoints = workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(run_ids.iter().copied()))
            .all(db)
            .await
            .map_err(manga_record_database("读取漫画续跑检查点范围失败"))?;
        let ids = checkpoints
            .into_iter()
            .filter(|checkpoint| {
                keys.contains(&checkpoint.step_key)
                    || (deletes_image
                        && checkpoint
                            .input_fingerprint_json
                            .as_ref()
                            .is_some_and(|input| {
                                input["image_type"].as_str() == Some("manga_page")
                                    && input["chapter_id"].as_i64() == Some(chapter_id)
                                    && page.is_none_or(|page| {
                                        input["panel_id"].as_str()
                                            == Some(format!("page{page}").as_str())
                                    })
                            }))
            })
            .map(|checkpoint| checkpoint.id)
            .collect::<Vec<_>>();
        deletion = deletion.filter(workflow_checkpoint::Column::Id.is_in(ids));
    }
    deletion
        .exec(db)
        .await
        .map_err(manga_record_database("清理漫画续跑检查点失败"))?;
    let remaining = workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::WorkflowRunId)
        .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(run_ids.iter().copied()))
        .filter(workflow_checkpoint::Column::Status.eq(crate::workflow::WORKFLOW_STATUS_COMPLETED))
        .distinct()
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(manga_record_database("读取剩余漫画检查点失败"))?;
    let exhausted = run_ids
        .iter()
        .copied()
        .filter(|id| !remaining.contains(id))
        .collect::<Vec<_>>();
    if !exhausted.is_empty() {
        workflow_run::Entity::update_many()
            .col_expr(workflow_run::Column::CanResume, Expr::value(false))
            .col_expr(
                workflow_run::Column::ResumeBlockReason,
                Expr::value("漫画产物已删除，已无可复用检查点，请重新生成"),
            )
            .col_expr(
                workflow_run::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(workflow_run::Column::Id.is_in(exhausted))
            .filter(workflow_run::Column::CanResume.eq(true))
            .exec(db)
            .await
            .map_err(manga_record_database("更新漫画续跑状态失败"))?;
    }
    Ok(())
}
