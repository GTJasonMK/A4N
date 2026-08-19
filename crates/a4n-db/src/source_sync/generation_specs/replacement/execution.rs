use a4n_core::models::{generation::generation_spec, source_sync::source_sync_run};
use sea_orm::ConnectionTrait;
use serde_json::Value;

use super::contracts::{
    SourceSyncReplacementSegmentRecordInput, SourceSyncReplacementSpecRecordInput,
};
use super::persistence::persist_replacement_generation_spec;
use super::preparation::prepare_replacement_generation_spec;
use crate::source_sync::{
    SourceSyncDbError, SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput, generation_spec_for_source_sync_run,
    source_sync_params_with_prompt_kind,
};

async fn create_replacement_generation_spec_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    source_spec: &generation_spec::Model,
    params_json: Value,
    replacement_segment: SourceSyncReplacementSegmentRecordInput,
    text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    media_refs: Vec<SourceSyncMediaRefRecordInput>,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let prepared = prepare_replacement_generation_spec(
        db,
        run,
        source_spec,
        &params_json,
        &replacement_segment,
    )
    .await?;
    persist_replacement_generation_spec(
        db,
        run,
        source_spec,
        params_json,
        replacement_segment,
        text_anchor_refs,
        entity_refs,
        media_refs,
        prepared,
    )
    .await
}

pub(in crate::source_sync) async fn create_source_sync_replacement_spec_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: SourceSyncReplacementSpecRecordInput,
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
    let source_spec =
        generation_spec_for_source_sync_run(db, run, input.generation_spec_id).await?;
    create_replacement_generation_spec_record(
        db,
        run,
        &source_spec,
        params_json,
        input.replacement_segment,
        input.text_anchor_refs,
        input.entity_refs,
        input.media_refs,
    )
    .await
}
