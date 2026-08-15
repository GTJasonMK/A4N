use a4n_core::models::source_sync::source_sync_run;
use a4n_domain::source_sync::{SourceSyncDecisionActor, SourceSyncResolutionStrategy};
use sea_orm::DatabaseTransaction;

use super::generation::{
    apply_create_replacement_decision, apply_delete_local_decision, apply_manual_decision,
    apply_rebind_anchor_decision, apply_refresh_decision,
};
use super::regeneration::{RegenerateSceneDecisionInput, apply_regenerate_scene_decision};
use crate::source_sync::decisions::types::PreparedSourceSyncDecisionRecordInput;
use crate::source_sync::{
    SourceSyncDbError, SourceSyncDecisionRecordInput, detach_derived_project_record,
    insert_source_sync_decision_record, resolve_source_sync_conflict_record,
    source_sync_conflict_for_decision,
};

pub(super) async fn apply_prepared_source_sync_decision_record(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    decision: PreparedSourceSyncDecisionRecordInput,
) -> Result<(), SourceSyncDbError> {
    let conflict = source_sync_conflict_for_decision(
        tx,
        run.id,
        decision.conflict_id(),
        "apply_source_sync_decision",
    )
    .await?;
    let strategy = decision.strategy();
    let mut decision_payload_json = decision.decision_payload_json().clone();
    if strategy == SourceSyncResolutionStrategy::Detach {
        detach_derived_project_record(tx, run.derived_project_id).await?;
    }

    match &decision {
        PreparedSourceSyncDecisionRecordInput::RebindAnchor {
            generation_spec_id,
            anchor_mappings,
            ..
        } => {
            apply_rebind_anchor_decision(
                tx,
                run,
                &conflict,
                strategy.as_str(),
                *generation_spec_id,
                anchor_mappings,
            )
            .await?;
        }
        PreparedSourceSyncDecisionRecordInput::Manual {
            prompt_artifact_id,
            prompt_text,
            ..
        } => {
            apply_manual_decision(
                tx,
                run,
                &conflict,
                strategy.as_str(),
                *prompt_artifact_id,
                prompt_text,
            )
            .await?;
        }
        PreparedSourceSyncDecisionRecordInput::DeleteLocal { .. } => {
            apply_delete_local_decision(tx, run, &conflict, strategy.as_str()).await?;
        }
        PreparedSourceSyncDecisionRecordInput::AdoptSource { input, .. }
        | PreparedSourceSyncDecisionRecordInput::RebuildPrompt { input, .. } => {
            apply_refresh_decision(tx, run, &conflict, strategy.as_str(), input).await?;
        }
        PreparedSourceSyncDecisionRecordInput::CreateReplacement { input, .. } => {
            apply_create_replacement_decision(tx, run, &conflict, strategy.as_str(), input).await?;
        }
        PreparedSourceSyncDecisionRecordInput::RegenerateScene {
            galgame_scene_id,
            reason,
            expected_scene_index,
            expected_source_snapshot_chapter_id,
            generated_scene,
            stage_outputs_json,
            ..
        } => {
            decision_payload_json = apply_regenerate_scene_decision(
                tx,
                run,
                &conflict,
                RegenerateSceneDecisionInput {
                    galgame_scene_id: *galgame_scene_id,
                    reason,
                    expected_scene_index: *expected_scene_index,
                    expected_source_snapshot_chapter_id: *expected_source_snapshot_chapter_id,
                    generated_scene,
                    stage_outputs_json,
                },
            )
            .await?;
        }
        PreparedSourceSyncDecisionRecordInput::KeepLocal { .. }
        | PreparedSourceSyncDecisionRecordInput::Detach { .. } => {}
    }

    resolve_source_sync_conflict_record(
        tx,
        conflict,
        strategy,
        decision.conflict_status(),
        decision_payload_json.clone(),
    )
    .await?;
    insert_source_sync_decision_record(
        tx,
        SourceSyncDecisionRecordInput {
            conflict_id: decision.conflict_id(),
            decided_by: SourceSyncDecisionActor::User,
            strategy,
            decision_payload_json,
        },
    )
    .await?;
    Ok(())
}
