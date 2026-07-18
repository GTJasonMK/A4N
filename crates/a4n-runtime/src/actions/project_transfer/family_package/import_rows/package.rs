use super::super::ProjectFamilyPackage;
use a4n_orchestrator::project::ProjectFamilyImportPackage;

pub(in crate::actions::project_transfer::family_package) fn project_family_import_package(
    package: &ProjectFamilyPackage,
) -> ProjectFamilyImportPackage {
    ProjectFamilyImportPackage {
        root_project_id: package.root_project_id,
        projects: package.projects.clone(),
        text_core: package.text_core.clone(),
        project_derivations: package.project_derivations.clone(),
        source_snapshots: package.source_snapshots.clone(),
        source_snapshot_chapters: package.source_snapshot_chapters.clone(),
        source_snapshot_outlines: package.source_snapshot_outlines.clone(),
        source_snapshot_profiles: package.source_snapshot_profiles.clone(),
        text_anchors: package.text_anchors.clone(),
        profile_evidence: package.profile_evidence.clone(),
        generation_specs: package.generation_specs.clone(),
        generation_spec_text_anchors: package.generation_spec_text_anchors.clone(),
        generation_spec_entity_refs: package.generation_spec_entity_refs.clone(),
        generation_spec_media_refs: package.generation_spec_media_refs.clone(),
        prompt_artifacts: package.prompt_artifacts.clone(),
        media_assets: package.media_assets.clone(),
        media_provenance: package.media_provenance.clone(),
        media_bindings: package.media_bindings.clone(),
        galgame_scripts: package.galgame_scripts.clone(),
        galgame_scenes: package.galgame_scenes.clone(),
        galgame_lines: package.galgame_lines.clone(),
        galgame_choices: package.galgame_choices.clone(),
        video_segments: package.video_segments.clone(),
        assembly_timelines: package.assembly_timelines.clone(),
        assembly_clips: package.assembly_clips.clone(),
        source_sync_runs: package.source_sync_runs.clone(),
        source_sync_conflicts: package.source_sync_conflicts.clone(),
        source_sync_decisions: package.source_sync_decisions.clone(),
    }
}
