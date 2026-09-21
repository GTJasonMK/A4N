use super::super::*;
use sea_orm::sea_query::{Alias, Expr};
use std::collections::HashSet;

pub async fn attach_runs_to_workflow_for_target<C>(
    db: &C,
    input: AttachGenerationRunsToWorkflowInput,
) -> Result<usize, GenerationDbError>
where
    C: ConnectionTrait,
{
    let target_table = required_owned_text(input.target_table, "target_table")?;
    validate_positive_id(input.target_id, "target_id")?;
    validate_positive_id(input.workflow_run_id, "workflow_run_id")?;
    let call_key = normalize_optional_owned_text(input.call_key);
    let input_fingerprint_sha256 = normalize_optional_owned_text(input.input_fingerprint_sha256);
    let run_ids = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::GenerationRunId)
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.eq(input.target_id))
        .order_by_asc(generation_run_target::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let run_ids = run_ids.into_iter().collect::<HashSet<_>>();
    if run_ids.is_empty() {
        return Ok(0);
    }
    let mut update = generation_run::Entity::update_many()
        .col_expr(
            generation_run::Column::WorkflowRunId,
            Expr::value(Some(input.workflow_run_id)),
        )
        .filter(generation_run::Column::Id.is_in(run_ids.iter().copied()));
    if let Some(call_key) = &call_key {
        update = update.col_expr(
            generation_run::Column::CallKey,
            Expr::value(call_key.clone()),
        );
    }
    if let Some(input_fingerprint_sha256) = &input_fingerprint_sha256 {
        update = update.col_expr(
            generation_run::Column::InputFingerprintSha256,
            Expr::value(input_fingerprint_sha256.clone()),
        );
    }
    if let Some(input_fingerprint_json) = &input.input_fingerprint_json {
        update = update.col_expr(
            generation_run::Column::InputFingerprintJson,
            Expr::value(input_fingerprint_json.clone()).cast_as(Alias::new("json")),
        );
    }
    if let Some(artifact_refs_json) = &input.artifact_refs_json {
        update = update.col_expr(
            generation_run::Column::ArtifactRefsJson,
            Expr::value(artifact_refs_json.clone()).cast_as(Alias::new("json")),
        );
    }
    let result = update.exec(db).await?;
    if result.rows_affected != run_ids.len() as u64 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "绑定 workflow 的生成运行数量不一致: updated={}, expected={}",
            result.rows_affected,
            run_ids.len()
        )));
    }
    Ok(run_ids.len())
}
