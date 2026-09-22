use super::contracts::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError,
    ProjectFamilyRootRow, ProjectFamilySnapshotRow,
};
use std::collections::HashMap;

pub(super) fn build_project_family_metadata(
    query: &ProjectFamilyMetadataQuery,
    roots_by_id: &HashMap<i64, ProjectFamilyRootRow>,
    snapshots_by_id: &HashMap<i64, ProjectFamilySnapshotRow>,
) -> Result<ProjectFamilyMetadata, ProjectFamilyMetadataReadError> {
    let mut metadata = ProjectFamilyMetadata {
        root_project_title: None,
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
        metadata.derived_chapter_scope = Some(snapshot.chapter_scope_json.clone());
    }
    Ok(metadata)
}
