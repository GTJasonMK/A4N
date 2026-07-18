use a4n_core::models::generation::generation_spec;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::lookup::find_generation_spec_by_key;
use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary, AssetLineageSyncError,
    persistence::{
        changes::{set_if_changed, set_string_if_changed, set_value_if_changed},
        types::GenerationSpecUpsertInput,
    },
    types::FamilyLineageContext,
};

pub(in crate::asset_lineage_sync) async fn upsert_generation_spec<C>(
    db: &C,
    family: &FamilyLineageContext,
    input: GenerationSpecUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<generation_spec::Model, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let existing = find_generation_spec_by_key(db, family, &input.canonical_key).await?;
    if let Some(existing) = existing {
        let mut active: generation_spec::ActiveModel = existing.clone().into();
        let mut changed = false;
        set_if_changed(
            &mut active.project_id,
            existing.project_id,
            input.project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.root_project_id,
            existing.root_project_id,
            input.root_project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_id,
            existing.source_snapshot_id,
            Some(input.source_snapshot_id),
            &mut changed,
        );
        set_if_changed(
            &mut active.source_chapter_id,
            existing.source_chapter_id,
            input.source_chapter_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_chapter_id,
            existing.source_snapshot_chapter_id,
            input.source_snapshot_chapter_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.target_kind,
            &existing.target_kind,
            input.target_kind,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.target_entity_kind,
            &existing.target_entity_kind,
            input.target_entity_kind,
            &mut changed,
        );
        set_if_changed(
            &mut active.target_entity_id,
            existing.target_entity_id,
            input.target_entity_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.status,
            &existing.status,
            input.status,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.source_version_hash,
            &existing.source_version_hash,
            input.source_version_hash,
            &mut changed,
        );
        set_value_if_changed(
            &mut active.params_json,
            &existing.params_json,
            input.params_json,
            &mut changed,
        );
        if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_generation_specs += 1;
            return active
                .update(db)
                .await
                .map_err(AssetLineageSyncError::Database);
        }
        return Ok(existing);
    }

    let spec = generation_spec::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(input.source_snapshot_id)),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        target_kind: Set(input.target_kind),
        target_entity_kind: Set(input.target_entity_kind),
        target_entity_id: Set(input.target_entity_id),
        status: Set(input.status),
        source_version_hash: Set(input.source_version_hash),
        params_json: Set(input.params_json),
        created_at: Set(input.created_at),
        updated_at: Set(input.updated_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    summary.created_generation_specs += 1;
    Ok(spec)
}
