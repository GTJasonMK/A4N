use std::collections::{HashMap, HashSet};

use super::super::super::ProjectFamilyPackage;
use super::super::manifest::SourceSnapshotManifestInfo;
use super::super::rules::{is_valid_snapshot_outline_kind, validate_snapshot_payload_identity};

pub(in super::super) fn validate_source_snapshot_outline_manifest(
    package: &ProjectFamilyPackage,
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for outline in &package.source_snapshot_outlines {
        validate_snapshot_payload_identity(
            snapshots,
            outline.snapshot_id,
            outline.root_project_id,
            outline.id,
            &outline.outline_kind,
            outline.source_outline_id,
            &outline.payload_json,
            &outline.payload_hash,
            "源快照大纲",
            "source_snapshot_outline_id",
            "outline_kind",
            "source_outline_id",
            is_valid_snapshot_outline_kind,
        )?;
        if !seen.insert((
            outline.snapshot_id,
            outline.outline_kind.clone(),
            outline.source_outline_id,
        )) {
            return Err(format!(
                "项目族导入包源快照大纲重复: snapshot_id={}, outline_kind={}, source_outline_id={}",
                outline.snapshot_id, outline.outline_kind, outline.source_outline_id
            ));
        }
    }
    Ok(())
}
