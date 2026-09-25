use crate::galgame::{GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED, GALGAME_SPEC_STATUS_DRAFT};
use a4n_db_models::models::generation::generation_spec;
use a4n_domain::galgame::GalgameGenerationSpecUpsertInput;

pub(super) fn new_galgame_generation_spec_model(
    input: GalgameGenerationSpecUpsertInput,
    source_version_hash: String,
    now: chrono::NaiveDateTime,
) -> generation_spec::Model {
    generation_spec::Model {
        id: 0,
        project_id: input.project_id,
        root_project_id: input.root_project_id,
        source_context_kind: Some(GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED.to_string()),
        source_snapshot_id: Some(input.source_snapshot_id),
        source_chapter_id: input.source_chapter_id,
        source_snapshot_chapter_id: input.source_snapshot_chapter_id,
        canvas_text_asset_id: None,
        target_kind: input.target_kind,
        target_entity_kind: input.target_entity_kind,
        target_entity_id: input.target_entity_id,
        status: GALGAME_SPEC_STATUS_DRAFT.to_string(),
        source_version_hash,
        params_json: input.params_json,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn updated_galgame_generation_spec_model(
    existing: &generation_spec::Model,
    input: GalgameGenerationSpecUpsertInput,
    source_version_hash: String,
    now: chrono::NaiveDateTime,
) -> generation_spec::Model {
    let mut model = existing.clone();
    model.root_project_id = input.root_project_id;
    model.source_context_kind = Some(GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED.to_string());
    model.source_snapshot_id = Some(input.source_snapshot_id);
    model.source_chapter_id = input.source_chapter_id;
    model.source_snapshot_chapter_id = input.source_snapshot_chapter_id;
    model.canvas_text_asset_id = None;
    model.target_kind = input.target_kind;
    model.target_entity_kind = input.target_entity_kind;
    model.target_entity_id = input.target_entity_id;
    model.status = GALGAME_SPEC_STATUS_DRAFT.to_string();
    model.source_version_hash = source_version_hash;
    model.params_json = input.params_json;
    model.updated_at = now;
    model
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
