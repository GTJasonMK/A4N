use super::{
    derived_domains::*,
    generation_media::*,
    snapshots::*,
    text_core::project_family_export_text_core_rows,
    types::{ProjectFamilyExportError, ProjectFamilyExportRows},
};
use sea_orm::ConnectionTrait;

pub async fn project_family_export_rows<C>(
    db: &C,
    root_project_id: i64,
    derived_project_ids: &[i64],
    derived_source_snapshot_ids: &[i64],
) -> Result<ProjectFamilyExportRows, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let text_core = project_family_export_text_core_rows(db, root_project_id).await?;
    let project_derivations =
        project_family_export_project_derivations(db, derived_project_ids).await?;
    let source_sync_runs = project_family_export_source_sync_runs(db, derived_project_ids).await?;
    let snapshot_ids = collect_project_family_export_snapshot_ids(
        derived_source_snapshot_ids,
        &project_derivations,
        &source_sync_runs,
    );
    let source_snapshots = project_family_export_source_snapshots(db, &snapshot_ids).await?;
    let source_snapshot_chapters =
        project_family_export_source_snapshot_chapters(db, &snapshot_ids).await?;
    let source_snapshot_outlines =
        project_family_export_source_snapshot_outlines(db, &snapshot_ids).await?;
    let source_snapshot_profiles =
        project_family_export_source_snapshot_profiles(db, &snapshot_ids).await?;
    let text_anchors =
        project_family_export_text_anchors(db, root_project_id, &snapshot_ids).await?;
    let profile_evidence = project_family_export_profile_evidence(db, &text_anchors).await?;

    let generation_specs = project_family_export_generation_specs(db, derived_project_ids).await?;
    let generation_spec_ids = generation_specs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let generation_spec_text_anchors =
        project_family_export_generation_spec_text_anchors(db, &generation_spec_ids).await?;
    let generation_spec_entity_refs =
        project_family_export_generation_spec_entity_refs(db, &generation_spec_ids).await?;
    let prompt_artifacts = project_family_export_prompt_artifacts(db, &generation_spec_ids).await?;

    let media_assets = project_family_export_media_assets(db, derived_project_ids).await?;
    let media_asset_ids = media_assets
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let generation_spec_media_refs = project_family_export_generation_spec_media_refs(
        db,
        &generation_spec_ids,
        &media_asset_ids,
    )
    .await?;
    let media_provenance = project_family_export_media_provenance(db, &media_asset_ids).await?;
    let media_bindings = project_family_export_media_bindings(db, &media_asset_ids).await?;

    let galgame_scripts = project_family_export_galgame_scripts(db, derived_project_ids).await?;
    let galgame_script_ids = galgame_scripts
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_scenes = project_family_export_galgame_scenes(db, &galgame_script_ids).await?;
    let galgame_scene_ids = galgame_scenes
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_lines = project_family_export_galgame_lines(db, &galgame_scene_ids).await?;
    let galgame_line_ids = galgame_lines
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_choices = project_family_export_galgame_choices(db, &galgame_line_ids).await?;

    let video_segments = project_family_export_video_segments(db, derived_project_ids).await?;
    let assembly_timelines =
        project_family_export_assembly_timelines(db, derived_project_ids).await?;
    let timeline_ids = assembly_timelines
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let assembly_clips = project_family_export_assembly_clips(db, &timeline_ids).await?;

    let sync_run_ids = source_sync_runs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let source_sync_conflicts =
        project_family_export_source_sync_conflicts(db, &sync_run_ids).await?;
    let conflict_ids = source_sync_conflicts
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let source_sync_decisions =
        project_family_export_source_sync_decisions(db, &conflict_ids).await?;

    Ok(ProjectFamilyExportRows {
        snapshot_ids,
        text_core,
        project_derivations,
        source_snapshots,
        source_snapshot_chapters,
        source_snapshot_outlines,
        source_snapshot_profiles,
        text_anchors,
        profile_evidence,
        generation_specs,
        generation_spec_text_anchors,
        generation_spec_entity_refs,
        generation_spec_media_refs,
        prompt_artifacts,
        media_assets,
        media_provenance,
        media_bindings,
        galgame_scripts,
        galgame_scenes,
        galgame_lines,
        galgame_choices,
        video_segments,
        assembly_timelines,
        assembly_clips,
        source_sync_runs,
        source_sync_conflicts,
        source_sync_decisions,
    })
}
