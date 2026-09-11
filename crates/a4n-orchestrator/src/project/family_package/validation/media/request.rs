mod fields;
mod galgame;
mod manifest;
mod targets;

use a4n_db_models::models::generation::generation_spec;

pub(super) use manifest::validate_media_request_snapshot_refs;

pub(super) fn is_galgame_visual_generation_spec(spec: &generation_spec::Model) -> bool {
    matches!(
        (spec.target_kind.as_str(), spec.target_entity_kind.as_str()),
        ("galgame_background", "galgame_scene") | ("galgame_cg", "galgame_line")
    )
}
