use crate::galgame::{
    GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED, GALGAME_SPEC_STATUS_DRAFT,
    GalgameAssetPlanSpecRecordError, GalgameGenerationSpecUpsert, GalgameGenerationSpecUpsertInput,
    GalgameGenerationSpecUpsertOutcome,
};
use a4n_core::models::generation::generation_spec;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) async fn upsert_galgame_generation_spec_record<C>(
    db: &C,
    existing_specs_by_canonical_key: &mut BTreeMap<String, generation_spec::Model>,
    input: GalgameGenerationSpecUpsertInput,
) -> Result<GalgameGenerationSpecUpsert, GalgameAssetPlanSpecRecordError>
where
    C: ConnectionTrait,
{
    let source_version_hash = galgame_generation_spec_source_hash_for_value(&input.params_json)?;
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing_specs_by_canonical_key
        .get(&input.canonical_key)
        .cloned()
    {
        let content_changed =
            galgame_generation_spec_content_changed(&existing, &input, &source_version_hash);
        if !content_changed {
            return Ok(GalgameGenerationSpecUpsert {
                model: existing,
                outcome: GalgameGenerationSpecUpsertOutcome::Reused,
            });
        }
        let mut active: generation_spec::ActiveModel = existing.into();
        active.root_project_id = Set(input.root_project_id);
        active.source_context_kind = Set(Some(
            GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED.to_string(),
        ));
        active.source_snapshot_id = Set(Some(input.source_snapshot_id));
        active.source_chapter_id = Set(input.source_chapter_id);
        active.source_snapshot_chapter_id = Set(input.source_snapshot_chapter_id);
        active.canvas_text_asset_id = Set(None);
        active.target_kind = Set(input.target_kind);
        active.target_entity_kind = Set(input.target_entity_kind);
        active.target_entity_id = Set(input.target_entity_id);
        active.status = Set(GALGAME_SPEC_STATUS_DRAFT.to_string());
        active.source_version_hash = Set(source_version_hash);
        active.params_json = Set(input.params_json);
        active.updated_at = Set(now);
        let canonical_key = input.canonical_key.clone();
        let model = active.update(db).await?;
        existing_specs_by_canonical_key.insert(canonical_key, model.clone());
        return Ok(GalgameGenerationSpecUpsert {
            model,
            outcome: GalgameGenerationSpecUpsertOutcome::Updated,
        });
    }

    let canonical_key = input.canonical_key.clone();
    let model = generation_spec::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_context_kind: Set(Some(
            GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED.to_string(),
        )),
        source_snapshot_id: Set(Some(input.source_snapshot_id)),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        canvas_text_asset_id: Set(None),
        target_kind: Set(input.target_kind),
        target_entity_kind: Set(input.target_entity_kind),
        target_entity_id: Set(input.target_entity_id),
        status: Set(GALGAME_SPEC_STATUS_DRAFT.to_string()),
        source_version_hash: Set(source_version_hash),
        params_json: Set(input.params_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    existing_specs_by_canonical_key.insert(canonical_key, model.clone());
    Ok(GalgameGenerationSpecUpsert {
        model,
        outcome: GalgameGenerationSpecUpsertOutcome::Created,
    })
}

pub fn galgame_generation_spec_content_changed(
    existing: &generation_spec::Model,
    input: &GalgameGenerationSpecUpsertInput,
    source_version_hash: &str,
) -> bool {
    existing.root_project_id != input.root_project_id
        || existing.source_context_kind.as_deref()
            != Some(GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED)
        || existing.source_snapshot_id != Some(input.source_snapshot_id)
        || existing.source_chapter_id != input.source_chapter_id
        || existing.source_snapshot_chapter_id != input.source_snapshot_chapter_id
        || existing.canvas_text_asset_id.is_some()
        || existing.target_kind != input.target_kind
        || existing.target_entity_kind != input.target_entity_kind
        || existing.target_entity_id != input.target_entity_id
        || existing.source_version_hash != source_version_hash
        || existing.params_json != input.params_json
}

pub fn galgame_generation_spec_source_hash_for_value(
    value: &Value,
) -> Result<String, serde_json::Error> {
    let serialized = serde_json::to_string(value)?;
    Ok(content_sha256(&serialized))
}

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
