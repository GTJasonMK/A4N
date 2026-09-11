use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::SourceSyncLocalStateKind;
use sea_orm::DatabaseTransaction;

use crate::source_sync::decisions::guards::{
    ensure_clean_source_sync_target, ensure_source_sync_decision_targets_generation_spec,
    source_sync_generation_spec_id_for_conflict,
};
use crate::source_sync::decisions::refresh::refresh_source_sync_generation_spec_record;
use crate::source_sync::decisions::types::SourceSyncRefreshSpecRecordInput;
use crate::source_sync::{
    SourceSyncAnchorMappingRecord, SourceSyncDbError, SourceSyncManualPromptArtifactRecordInput,
    SourceSyncReplacementSpecRecordInput, create_source_sync_replacement_spec_record,
    delete_local_generation_spec_record, insert_manual_source_sync_prompt_artifact_record,
    rebind_generation_spec_anchor_records, render_and_insert_source_sync_prompt_artifact_record,
};

pub(super) async fn apply_rebind_anchor_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    generation_spec_id: i64,
    anchor_mappings: &[SourceSyncAnchorMappingRecord],
) -> Result<(), SourceSyncDbError> {
    ensure_clean_source_sync_target(conflict, strategy)?;
    ensure_source_sync_decision_targets_generation_spec(conflict, generation_spec_id, strategy)?;
    if anchor_mappings.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(
            "decision_payload_json.anchor_mappings 不能为空".to_string(),
        ));
    }
    rebind_generation_spec_anchor_records(tx, run, generation_spec_id, anchor_mappings).await?;
    Ok(())
}

pub(super) async fn apply_manual_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    prompt_artifact_id: i64,
    prompt_text: &str,
) -> Result<(), SourceSyncDbError> {
    let expected_generation_spec_id =
        source_sync_generation_spec_id_for_conflict(conflict, strategy)?;
    insert_manual_source_sync_prompt_artifact_record(
        tx,
        run,
        SourceSyncManualPromptArtifactRecordInput {
            expected_generation_spec_id,
            prompt_artifact_id,
            prompt_text: prompt_text.to_string(),
            renderer_version: "source-sync-manual-v1".to_string(),
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn apply_delete_local_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<(), SourceSyncDbError> {
    if !SourceSyncLocalStateKind::parse(&conflict.local_state_kind).is_some_and(|state| {
        matches!(
            state,
            SourceSyncLocalStateKind::Clean | SourceSyncLocalStateKind::Draft
        )
    }) {
        return Err(SourceSyncDbError::InvalidArgument(
            "delete_local 只能处理 clean 或 draft 本地对象".to_string(),
        ));
    }
    let generation_spec_id = source_sync_generation_spec_id_for_conflict(conflict, strategy)?;
    delete_local_generation_spec_record(tx, run, generation_spec_id).await?;
    Ok(())
}

pub(super) async fn apply_refresh_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    input: &SourceSyncRefreshSpecRecordInput,
) -> Result<(), SourceSyncDbError> {
    ensure_clean_source_sync_target(conflict, strategy)?;
    ensure_source_sync_decision_targets_generation_spec(
        conflict,
        input.generation_spec_id,
        strategy,
    )?;
    let updated = refresh_source_sync_generation_spec_record(tx, run, input.clone()).await?;
    render_and_insert_source_sync_prompt_artifact_record(
        tx,
        &updated,
        &format!("source-sync-{strategy}-v1"),
    )
    .await?;
    Ok(())
}

pub(super) async fn apply_create_replacement_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    input: &SourceSyncReplacementSpecRecordInput,
) -> Result<(), SourceSyncDbError> {
    ensure_source_sync_decision_targets_generation_spec(
        conflict,
        input.generation_spec_id,
        strategy,
    )?;
    let replacement = create_source_sync_replacement_spec_record(tx, run, input.clone()).await?;
    render_and_insert_source_sync_prompt_artifact_record(
        tx,
        &replacement,
        "source-sync-create_replacement-v1",
    )
    .await?;
    Ok(())
}
