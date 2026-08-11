use super::contracts::{AppliedCharacterSyncPersistenceRecord, CharacterSyncPersistenceInput};
use super::{snapshots, validation::validate_character_sync_persistence_input, write_steps};
use crate::character::{
    CharacterDbError, CharacterProfileCurrentStateUpdate, character_attribute_ownership,
    delete_ai_timeline_artifacts_from_chapter_checked, profile_current_state_fields,
    require_profile, require_profile_for_update, rewind_ai_deletion_marks_after,
    update_profile_current_state,
};
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn apply_character_sync_persistence_record<C>(
    db: &C,
    mut input: CharacterSyncPersistenceInput,
) -> Result<AppliedCharacterSyncPersistenceRecord, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_character_sync_persistence_input(&input)?;

    let tx = db.begin().await?;
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(
        &tx,
        &input.source_chapters,
    )
    .await?
    {
        return Err(CharacterDbError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: format!(
                "保存第 {} 章角色同步结果；章节正文已变化，请重新分析后再同步",
                input.target_chapter
            ),
        });
    }
    let profile = require_profile_for_update(&tx, input.profile_id).await?;
    if profile.project_id != input.project_id {
        return Err(CharacterDbError::InvalidArgument(format!(
            "角色同步 profile/project 不匹配: profile_id={}, expected_project={}, actual_project={}",
            input.profile_id, profile.project_id, input.project_id
        )));
    }
    ensure_character_sync_profile_source_matches(&profile, &input)?;
    let ownership = character_attribute_ownership(&tx, &profile).await?;

    let deleted_timeline = delete_ai_timeline_artifacts_from_chapter_checked(
        &tx,
        input.profile_id,
        input.start_chapter,
        input.clear_to_chapter,
    )
    .await?;
    let deletion_mark_rewind =
        rewind_ai_deletion_marks_after(&tx, input.profile_id, input.start_chapter - 1).await?;

    let baseline_fields = profile_current_state_fields(input.baseline_state.clone());
    update_profile_current_state(
        &tx,
        profile,
        CharacterProfileCurrentStateUpdate {
            gender: baseline_fields.gender,
            age: baseline_fields.age,
            appearance: baseline_fields.appearance,
            background: baseline_fields.background,
            personality: baseline_fields.personality,
            current_state: baseline_fields.current_state,
            last_synced_chapter: None,
        },
    )
    .await?;

    let write_steps = std::mem::take(&mut input.write_steps);
    let write_summary = write_steps::apply_character_sync_write_steps_in(
        &tx,
        input.profile_id,
        write_steps,
        &ownership,
    )
    .await?;
    let stored_snapshots = snapshots::store_character_sync_snapshots_in(
        &tx,
        &input,
        write_summary.implicit_reviews_applied,
    )
    .await?;

    let target_profile = require_profile(&tx, input.profile_id).await?;
    let target_fields = profile_current_state_fields(stored_snapshots.target_attributes.clone());
    update_profile_current_state(
        &tx,
        target_profile,
        CharacterProfileCurrentStateUpdate {
            gender: target_fields.gender,
            age: target_fields.age,
            appearance: target_fields.appearance,
            background: target_fields.background,
            personality: target_fields.personality,
            current_state: target_fields.current_state,
            last_synced_chapter: Some(input.target_chapter),
        },
    )
    .await?;

    snapshots::enrich_character_sync_generation_run(
        &mut input.generation_run,
        input.project_id,
        input.profile_id,
        &write_summary,
        stored_snapshots.state_indexes.len(),
    )?;
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        &tx,
        input.generation_run,
        crate::generation::GenerationRunCheckpointContext::default(),
    )
    .await?;

    tx.commit().await?;
    Ok(AppliedCharacterSyncPersistenceRecord {
        deleted_timeline,
        deletion_mark_rewind,
        write_summary,
        target_snapshot_hash: stored_snapshots.target_hash,
        state_indexes: stored_snapshots.state_indexes,
    })
}

fn ensure_character_sync_profile_source_matches(
    profile: &a4n_core::models::character::protagonist_profile::Model,
    input: &CharacterSyncPersistenceInput,
) -> Result<(), CharacterDbError> {
    if profile.name == input.profile_name
        && profile.updated_at == input.expected_profile_updated_at
        && profile.last_synced_chapter == input.expected_profile_last_synced_chapter
        && profile.base_attributes == input.expected_profile_base_attributes
        && profile.current_state == input.expected_profile_current_state
    {
        return Ok(());
    }
    Err(CharacterDbError::InvalidState {
        current: "character_profile_source_changed".to_string(),
        operation: format!(
            "保存第 {} 章角色同步结果；角色正式资料已变化，请按当前资料重新同步",
            input.target_chapter
        ),
    })
}
