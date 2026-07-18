mod background;
mod cg;
mod portrait;

use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::{source_snapshot_chapter, source_snapshot_profile},
};
use std::collections::HashMap;

use background::validate_galgame_background_spec_params;
use cg::validate_galgame_cg_spec_params;
use portrait::validate_galgame_portrait_spec_params;

pub(super) fn validate_galgame_generation_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
        ("galgame_background", "galgame_scene") => validate_galgame_background_spec_params(
            spec,
            source_snapshot_id,
            snapshot_chapters,
            galgame_scripts,
            galgame_scenes,
        ),
        ("galgame_cg", "galgame_line") => validate_galgame_cg_spec_params(
            spec,
            source_snapshot_id,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        ("portrait_expression", "character_profile") => validate_galgame_portrait_spec_params(
            spec,
            source_snapshot_id,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        _ => Ok(()),
    }
}
