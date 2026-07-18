use std::collections::HashMap;

use super::super::ProjectFamilyPackage;

use super::chapters::validate_source_snapshot_chapter_manifest;
use super::outlines_profiles::{
    validate_source_snapshot_outline_manifest, validate_source_snapshot_profile_manifest,
};
use super::rules::parse_chapter_scope_json;

pub(in crate::actions::project_transfer::family_package::validation) fn validate_source_snapshot_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let mut snapshots = HashMap::new();
    for snapshot in &package.source_snapshots {
        if snapshot.root_project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包源文本快照不属于根项目: source_snapshot_id={}, root_project_id={}",
                snapshot.id, snapshot.root_project_id
            ));
        }
        let (start_chapter, end_chapter) = parse_chapter_scope_json(
            &snapshot.chapter_scope_json,
            "源文本快照 chapter_scope_json",
        )?;
        snapshots.insert(
            snapshot.id,
            SourceSnapshotManifestInfo {
                root_project_id: snapshot.root_project_id,
                start_chapter,
                end_chapter,
            },
        );
    }
    validate_source_snapshot_chapter_manifest(package, &snapshots)?;
    validate_source_snapshot_outline_manifest(package, &snapshots)?;
    validate_source_snapshot_profile_manifest(package, &snapshots)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct SourceSnapshotManifestInfo {
    pub(super) root_project_id: i64,
    pub(super) start_chapter: i32,
    pub(super) end_chapter: i32,
}
