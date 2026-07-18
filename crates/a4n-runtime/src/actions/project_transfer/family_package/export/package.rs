use std::collections::HashSet;

use a4n_core::models::project::{novel_project, project_source_snapshot};
use chrono::Utc;

use crate::state::AppState;

use super::super::ProjectFamilyPackage;
use super::media::collect_media_files;
use super::selection::validate_same_user;

pub(super) async fn build_export_package(
    state: &AppState,
    root: novel_project::Model,
    derived_projects: Vec<novel_project::Model>,
    mode: &str,
) -> Result<ProjectFamilyPackage, String> {
    validate_same_user(&root, &derived_projects)?;
    let derived_project_ids = derived_projects
        .iter()
        .map(|project| project.id)
        .collect::<Vec<_>>();
    let derived_source_snapshot_ids = derived_projects
        .iter()
        .filter_map(|project| project.source_snapshot_id)
        .collect::<Vec<_>>();
    let mut projects = Vec::with_capacity(1 + derived_projects.len());
    projects.push(root.clone());
    projects.extend(derived_projects.clone());

    let export_rows = load_project_family_export_rows(
        state,
        root.id,
        &derived_project_ids,
        &derived_source_snapshot_ids,
    )
    .await?;
    validate_snapshot_coverage(&export_rows.snapshot_ids, &export_rows.source_snapshots)?;
    let media_files = collect_media_files(state, &export_rows.media_assets)?;
    Ok(ProjectFamilyPackage {
        version: super::super::PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: mode.to_string(),
        root_project_id: root.id,
        projects,
        text_core: export_rows.text_core,
        project_derivations: export_rows.project_derivations,
        source_snapshots: export_rows.source_snapshots,
        source_snapshot_chapters: export_rows.source_snapshot_chapters,
        source_snapshot_outlines: export_rows.source_snapshot_outlines,
        source_snapshot_profiles: export_rows.source_snapshot_profiles,
        text_anchors: export_rows.text_anchors,
        profile_evidence: export_rows.profile_evidence,
        generation_specs: export_rows.generation_specs,
        generation_spec_text_anchors: export_rows.generation_spec_text_anchors,
        generation_spec_entity_refs: export_rows.generation_spec_entity_refs,
        generation_spec_media_refs: export_rows.generation_spec_media_refs,
        prompt_artifacts: export_rows.prompt_artifacts,
        media_assets: export_rows.media_assets,
        media_provenance: export_rows.media_provenance,
        media_bindings: export_rows.media_bindings,
        galgame_scripts: export_rows.galgame_scripts,
        galgame_scenes: export_rows.galgame_scenes,
        galgame_lines: export_rows.galgame_lines,
        galgame_choices: export_rows.galgame_choices,
        video_segments: export_rows.video_segments,
        assembly_timelines: export_rows.assembly_timelines,
        assembly_clips: export_rows.assembly_clips,
        source_sync_runs: export_rows.source_sync_runs,
        source_sync_conflicts: export_rows.source_sync_conflicts,
        source_sync_decisions: export_rows.source_sync_decisions,
        media_files,
    })
}

async fn load_project_family_export_rows(
    state: &AppState,
    root_project_id: i64,
    derived_project_ids: &[i64],
    derived_source_snapshot_ids: &[i64],
) -> Result<a4n_orchestrator::project::ProjectFamilyExportRows, String> {
    state
        .orchestrator
        .project_family_export_rows(
            root_project_id,
            derived_project_ids,
            derived_source_snapshot_ids,
        )
        .await
        .map_err(crate::actions::app_error_message)
}

fn validate_snapshot_coverage(
    expected_ids: &[i64],
    snapshots: &[project_source_snapshot::Model],
) -> Result<(), String> {
    let found = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    for id in expected_ids {
        if !found.contains(id) {
            return Err(format!("派生项目引用的源文本快照不存在: {id}"));
        }
    }
    Ok(())
}
