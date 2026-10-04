mod background;
mod cg;
mod portrait;

use std::collections::HashMap;

use background::validate_galgame_background_spec_params;
use cg::validate_galgame_cg_spec_params;
use portrait::validate_galgame_portrait_spec_params;

pub(super) fn validate_galgame_generation_spec_params(
    spec: &a4n_domain::project::ProjectFamilyGenerationSpecRecord,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
    snapshot_profiles: &HashMap<
        (i64, String),
        &a4n_domain::project::ProjectFamilySourceSnapshotProfileRecord,
    >,
    galgame_scripts: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    galgame_scenes: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    galgame_lines: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameLineRecord>,
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
            snapshot_chapters,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        ("portrait_expression", "character_profile") => validate_galgame_portrait_spec_params(
            spec,
            source_snapshot_id,
            snapshot_chapters,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        _ => Ok(()),
    }
}
