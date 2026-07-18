use super::{
    super::contracts::SourceSyncReferenceSets,
    fields::{
        validate_optional_source_sync_json_id_ref, validate_optional_source_sync_profile_ref,
    },
    relationships::validate_source_sync_galgame_line_scene_pair,
};
use serde_json::{Map, Value};

pub(super) fn validate_source_sync_source_summary_refs(
    conflict_scope: &str,
    source_summary: &Map<String, Value>,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    if conflict_scope == "text_anchor" {
        validate_optional_source_sync_json_id_ref(
            source_summary,
            "old_text_anchor_id",
            &refs.text_anchor_ids,
            "文本锚点",
            "source_summary_json",
            conflict_id,
        )?;
        validate_optional_source_sync_json_id_ref(
            source_summary,
            "new_text_anchor_id",
            &refs.text_anchor_ids,
            "文本锚点",
            "source_summary_json",
            conflict_id,
        )?;
    }
    if conflict_scope == "profile" {
        validate_optional_source_sync_profile_ref(
            source_summary,
            "source_summary_json",
            conflict_id,
            refs,
        )?;
    }
    Ok(())
}

pub(super) fn validate_source_sync_local_summary_refs(
    local_summary: &Map<String, Value>,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    validate_optional_source_sync_json_id_ref(
        local_summary,
        "generation_spec_id",
        &refs.generation_spec_ids,
        "生成意图",
        "local_summary_json",
        conflict_id,
    )?;
    let galgame_scene_id = validate_optional_source_sync_json_id_ref(
        local_summary,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        "local_summary_json",
        conflict_id,
    )?;
    let galgame_line_id = validate_optional_source_sync_json_id_ref(
        local_summary,
        "galgame_line_id",
        &refs.galgame_line_ids,
        "Galgame 行",
        "local_summary_json",
        conflict_id,
    )?;
    validate_source_sync_galgame_line_scene_pair(
        galgame_line_id,
        galgame_scene_id,
        "local_summary_json",
        conflict_id,
        refs,
    )
}
