use super::super::super::super::ProjectFamilyPackage;
use crate::project::validate_import_unique_ids;

macro_rules! validate_id_groups {
    ($(($records:expr, $field:ident, $label:literal)),+ $(,)?) => {
        $(
            validate_import_unique_ids(($records).iter().map(|record| record.$field), $label)?;
        )+
    };
}

pub(super) fn validate_primary_row_ids(package: &ProjectFamilyPackage) -> Result<(), String> {
    if let Some(blueprint) = &package.text_core.blueprint {
        validate_import_unique_ids(std::iter::once(blueprint.id), "text_core.blueprint.id")?;
    }
    validate_id_groups!(
        (
            &package.text_core.blueprint_versions,
            id,
            "text_core.blueprint_versions.id"
        ),
        (
            &package.text_core.blueprint_characters,
            id,
            "text_core.blueprint_characters.id"
        ),
        (
            &package.text_core.blueprint_relationships,
            id,
            "text_core.blueprint_relationships.id"
        ),
        (
            &package.text_core.part_outlines,
            id,
            "text_core.part_outlines.id"
        ),
        (
            &package.text_core.chapter_outlines,
            id,
            "text_core.chapter_outlines.id"
        ),
        (&package.text_core.chapters, id, "text_core.chapters.id"),
        (
            &package.text_core.character_profiles,
            id,
            "text_core.character_profiles.id"
        ),
        (
            &package.text_core.object_profiles,
            id,
            "text_core.object_profiles.id"
        ),
        (
            &package.text_core.scene_profiles,
            id,
            "text_core.scene_profiles.id"
        ),
        (
            &package.text_core.scene_occurrences,
            id,
            "text_core.scene_occurrences.id"
        ),
        (
            &package.text_core.scene_transitions,
            id,
            "text_core.scene_transitions.id"
        ),
        (
            &package.text_core.scene_entity_states,
            id,
            "text_core.scene_entity_states.id"
        ),
        (&package.source_snapshots, id, "source_snapshots.id"),
        (
            &package.source_snapshot_chapters,
            id,
            "source_snapshot_chapters.id"
        ),
        (
            &package.source_snapshot_outlines,
            id,
            "source_snapshot_outlines.id"
        ),
        (
            &package.source_snapshot_profiles,
            id,
            "source_snapshot_profiles.id"
        ),
        (&package.text_anchors, id, "text_anchors.id"),
        (&package.profile_evidence, id, "profile_evidence.id"),
        (&package.galgame_scripts, id, "galgame_scripts.id"),
        (&package.galgame_scenes, id, "galgame_scenes.id"),
        (&package.galgame_lines, id, "galgame_lines.id"),
        (&package.galgame_choices, id, "galgame_choices.id"),
        (&package.generation_specs, id, "generation_specs.id"),
    );
    Ok(())
}

pub(super) fn validate_secondary_row_ids(package: &ProjectFamilyPackage) -> Result<(), String> {
    validate_id_groups!(
        (&package.prompt_artifacts, id, "prompt_artifacts.id"),
        (&package.media_assets, id, "media_assets.id"),
        (
            &package.media_provenance,
            media_asset_id,
            "media_provenance.media_asset_id"
        ),
        (&package.media_bindings, id, "media_bindings.id"),
        (&package.video_segments, id, "video_segments.id"),
        (&package.assembly_timelines, id, "assembly_timelines.id"),
        (&package.assembly_clips, id, "assembly_clips.id"),
        (&package.source_sync_runs, id, "source_sync_runs.id"),
        (
            &package.source_sync_conflicts,
            id,
            "source_sync_conflicts.id"
        ),
        (
            &package.source_sync_decisions,
            id,
            "source_sync_decisions.id"
        ),
        (
            &package.source_sync_decisions,
            conflict_id,
            "source_sync_decisions.conflict_id"
        ),
    );
    Ok(())
}
