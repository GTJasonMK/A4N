use super::types::SourceSyncRefreshSpecRecordInput;
use crate::source_sync::{
    SourceSyncDbError, generation_spec_for_source_sync_run, replace_generation_spec_ref_records,
    source_sync_generation_db_error, source_sync_params_with_prompt_kind,
    update_generation_spec_params_record, update_generation_spec_to_snapshot_record,
};
use a4n_db_models::models::{generation::generation_spec, source_sync::source_sync_run};
use sea_orm::ConnectionTrait;

pub(super) async fn refresh_source_sync_generation_spec_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: SourceSyncRefreshSpecRecordInput,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let params_json = source_sync_params_with_prompt_kind(input.params_json, &input.prompt_kind)?;
    if input.text_anchor_refs.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(
            "decision_payload_json.text_anchor_refs 不能为空".to_string(),
        ));
    }
    let spec = generation_spec_for_source_sync_run(db, run, input.generation_spec_id).await?;
    replace_generation_spec_ref_records(
        db,
        &spec,
        run.to_snapshot_id,
        input.text_anchor_refs,
        input.entity_refs,
        input.media_refs,
    )
    .await?;
    let updated = update_generation_spec_to_snapshot_record(db, &spec, run.to_snapshot_id).await?;
    let updated = update_generation_spec_params_record(db, updated, params_json).await?;
    crate::generation::validate_required_prompt_refs(db, &updated)
        .await
        .map_err(source_sync_generation_db_error)?;
    Ok(updated)
}
