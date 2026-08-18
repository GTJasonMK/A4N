use super::contracts::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError,
};
use a4n_core::models::project::{novel_project, project_source_snapshot};
use std::collections::HashMap;

pub(in crate::project::family_metadata) fn build_project_family_metadata(
    query: &ProjectFamilyMetadataQuery,
    roots_by_id: &HashMap<i64, novel_project::Model>,
    snapshots_by_id: &HashMap<i64, project_source_snapshot::Model>,
) -> Result<ProjectFamilyMetadata, ProjectFamilyMetadataReadError> {
    let mut metadata = ProjectFamilyMetadata {
        project_id: query.project_id,
        root_project_title: None,
        source_snapshot_created_at: None,
        derived_chapter_scope: None,
    };
    if let Some(root_project_id) = query.root_project_id {
        let root = roots_by_id
            .get(&root_project_id)
            .ok_or(ProjectFamilyMetadataReadError::RootProjectMissing { root_project_id })?;
        metadata.root_project_title = Some(root.title.clone());
    }
    if let Some(source_snapshot_id) = query.source_snapshot_id {
        let snapshot = snapshots_by_id
            .get(&source_snapshot_id)
            .ok_or(ProjectFamilyMetadataReadError::SourceSnapshotMissing { source_snapshot_id })?;
        metadata.source_snapshot_created_at = Some(snapshot.created_at.to_string());
        metadata.derived_chapter_scope = Some(snapshot.chapter_scope_json.clone());
    }
    Ok(metadata)
}
