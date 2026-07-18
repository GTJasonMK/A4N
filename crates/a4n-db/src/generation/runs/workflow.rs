use super::super::*;

pub async fn attach_runs_to_workflow_for_target<C>(
    db: &C,
    input: AttachGenerationRunsToWorkflowInput,
) -> Result<Vec<generation_run::Model>, GenerationDbError>
where
    C: ConnectionTrait,
{
    let target_table = required_owned_text(input.target_table, "target_table")?;
    validate_positive_id(input.target_id, "target_id")?;
    validate_positive_id(input.workflow_run_id, "workflow_run_id")?;
    let call_key = normalize_optional_owned_text(input.call_key);
    let input_fingerprint_sha256 = normalize_optional_owned_text(input.input_fingerprint_sha256);
    let targets = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.eq(input.target_id))
        .order_by_asc(generation_run_target::Column::Id)
        .all(db)
        .await?;
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let run_ids = targets
        .iter()
        .map(|target| target.generation_run_id)
        .collect::<BTreeSet<_>>();
    let runs = generation_run::Entity::find()
        .filter(generation_run::Column::Id.is_in(run_ids))
        .order_by_asc(generation_run::Column::CreatedAt)
        .all(db)
        .await?;
    let mut updated = Vec::with_capacity(runs.len());
    for run in runs {
        let mut active: generation_run::ActiveModel = run.into();
        active.workflow_run_id = Set(Some(input.workflow_run_id));
        if call_key.is_some() {
            active.call_key = Set(call_key.clone());
        }
        if input_fingerprint_sha256.is_some() {
            active.input_fingerprint_sha256 = Set(input_fingerprint_sha256.clone());
        }
        if input.input_fingerprint_json.is_some() {
            active.input_fingerprint_json = Set(input.input_fingerprint_json.clone());
        }
        if input.artifact_refs_json.is_some() {
            active.artifact_refs_json = Set(input.artifact_refs_json.clone());
        }
        updated.push(active.update(db).await?);
    }
    Ok(updated)
}
