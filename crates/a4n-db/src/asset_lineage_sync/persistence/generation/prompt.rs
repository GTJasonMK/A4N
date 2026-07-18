use a4n_core::models::generation::prompt_artifact;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary, AssetLineageSyncError, persistence::changes::set_string_if_changed,
};

const RENDERER_VERSION: &str = "legacy_asset_lineage_sync_v1";

pub(in crate::asset_lineage_sync) async fn upsert_prompt_artifact<C>(
    db: &C,
    generation_spec_id: i64,
    prompt_kind: &str,
    prompt_text: &str,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<prompt_artifact::Model, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let existing = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(generation_spec_id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    if let Some(existing) = existing {
        let mut active: prompt_artifact::ActiveModel = existing.clone().into();
        let mut changed = false;
        set_string_if_changed(
            &mut active.prompt_kind,
            &existing.prompt_kind,
            prompt_kind.to_string(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.prompt_text,
            &existing.prompt_text,
            prompt_text.to_string(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.renderer_version,
            &existing.renderer_version,
            RENDERER_VERSION.to_string(),
            &mut changed,
        );
        if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_prompt_artifacts += 1;
            return active
                .update(db)
                .await
                .map_err(AssetLineageSyncError::Database);
        }
        return Ok(existing);
    }

    let artifact = prompt_artifact::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        prompt_kind: Set(prompt_kind.to_string()),
        prompt_text: Set(prompt_text.to_string()),
        renderer_version: Set(RENDERER_VERSION.to_string()),
        edited_by_user: Set(false),
        created_at: Set(created_at),
        updated_at: Set(updated_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    summary.created_prompt_artifacts += 1;
    Ok(artifact)
}
