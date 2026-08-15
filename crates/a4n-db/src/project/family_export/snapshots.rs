use a4n_core::models::{
    project::{
        profile_evidence, project_source_snapshot, source_snapshot_chapter,
        source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeSet;

use super::types::ProjectFamilyExportError;

pub(super) fn collect_project_family_export_snapshot_ids(
    derived_source_snapshot_ids: &[i64],
    sync_runs: &[source_sync_run::Model],
) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for id in derived_source_snapshot_ids {
        ids.insert(*id);
    }
    for run in sync_runs {
        ids.insert(run.from_snapshot_id);
        ids.insert(run.to_snapshot_id);
    }
    ids.into_iter().collect()
}

project_family_export_load_by_ids!(
    project_family_export_source_snapshots,
    project_source_snapshot::Entity,
    project_source_snapshot::Column::Id,
    project_source_snapshot::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_source_snapshot_chapters,
    source_snapshot_chapter::Entity,
    source_snapshot_chapter::Column::SnapshotId,
    source_snapshot_chapter::Column::SnapshotId,
    source_snapshot_chapter::Column::ChapterNumber,
);

project_family_export_load_by_ids!(
    project_family_export_source_snapshot_outlines,
    source_snapshot_outline::Entity,
    source_snapshot_outline::Column::SnapshotId,
    source_snapshot_outline::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_source_snapshot_profiles,
    source_snapshot_profile::Entity,
    source_snapshot_profile::Column::SnapshotId,
    source_snapshot_profile::Column::Id,
);

pub(super) async fn project_family_export_text_anchors<C>(
    db: &C,
    root_project_id: i64,
    snapshot_ids: &[i64],
) -> Result<Vec<text_anchor::Model>, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let mut query =
        text_anchor::Entity::find().filter(text_anchor::Column::ProjectId.eq(root_project_id));
    if !snapshot_ids.is_empty() {
        query = query.filter(
            Condition::any()
                .add(text_anchor::Column::SourceSnapshotId.is_null())
                .add(text_anchor::Column::SourceSnapshotId.is_in(snapshot_ids.to_vec())),
        );
    }
    query
        .order_by_asc(text_anchor::Column::Id)
        .all(db)
        .await
        .map_err(ProjectFamilyExportError::Database)
}

pub(super) async fn project_family_export_profile_evidence<C>(
    db: &C,
    anchors: &[text_anchor::Model],
) -> Result<Vec<profile_evidence::Model>, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let anchor_ids = anchors.iter().map(|anchor| anchor.id).collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Ok(Vec::new());
    }
    profile_evidence::Entity::find()
        .filter(profile_evidence::Column::TextAnchorId.is_in(anchor_ids))
        .order_by_asc(profile_evidence::Column::Id)
        .all(db)
        .await
        .map_err(ProjectFamilyExportError::Database)
}
