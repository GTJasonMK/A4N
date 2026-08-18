use std::collections::BTreeMap;

use a4n_core::models::project::{novel_project, project_source_snapshot, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::asset_lineage_sync::{
    AssetLineageSyncError,
    types::{FamilyLineageContext, ProjectLineageContext},
};

pub(in crate::asset_lineage_sync) async fn load_family_lineage_context<C>(
    db: &C,
    project_id: i64,
) -> Result<FamilyLineageContext, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let requested = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AssetLineageSyncError::Database)?
        .ok_or_else(|| AssetLineageSyncError::NotFound(format!("项目不存在: {project_id}")))?;
    let root_project_id = requested.root_project_id.unwrap_or(requested.id);
    let mut projects = novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .order_by_asc(novel_project::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    let root_project = novel_project::Entity::find_by_id(root_project_id)
        .one(db)
        .await
        .map_err(AssetLineageSyncError::Database)?
        .ok_or_else(|| {
            AssetLineageSyncError::NotFound(format!("根文本项目不存在: {root_project_id}"))
        })?;
    projects.push(root_project);
    if !projects.iter().any(|project| project.id == requested.id) {
        projects.push(requested);
    }
    projects.sort_by_key(|project| project.id);
    projects.dedup_by_key(|project| project.id);

    let snapshots = project_source_snapshot::Entity::find()
        .filter(project_source_snapshot::Column::RootProjectId.eq(root_project_id))
        .order_by_desc(project_source_snapshot::Column::CreatedAt)
        .order_by_desc(project_source_snapshot::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    let snapshots_by_id = snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot.clone()))
        .collect::<BTreeMap<_, _>>();
    let latest_snapshot = snapshots.first().cloned();
    let snapshot_ids = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<Vec<_>>();
    let snapshot_chapters = if snapshot_ids.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.is_in(snapshot_ids))
            .order_by_asc(source_snapshot_chapter::Column::SnapshotId)
            .order_by_asc(source_snapshot_chapter::Column::SourceChapterId)
            .all(db)
            .await
            .map_err(AssetLineageSyncError::Database)?
    };
    let mut snapshot_chapters_by_snapshot_id =
        BTreeMap::<i64, BTreeMap<i64, source_snapshot_chapter::Model>>::new();
    for chapter in snapshot_chapters {
        snapshot_chapters_by_snapshot_id
            .entry(chapter.snapshot_id)
            .or_default()
            .insert(chapter.source_chapter_id, chapter);
    }

    let mut project_ids = Vec::with_capacity(projects.len());
    let mut contexts_by_project_id = BTreeMap::new();
    for project in projects {
        project_ids.push(project.id);
        let source_mode = project.source_mode().map_err(|error| {
            AssetLineageSyncError::InvalidArgument(format!("项目 {}: {error}", project.id))
        })?;
        let selected_snapshot = match source_mode {
            ProjectSourceMode::Freeform => None,
            ProjectSourceMode::Self_ => latest_snapshot.clone(),
            ProjectSourceMode::Derived => project
                .source_snapshot_id
                .and_then(|id| snapshots_by_id.get(&id).cloned()),
        };
        let snapshot_chapters_by_source = selected_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot_chapters_by_snapshot_id.get(&snapshot.id).cloned())
            .unwrap_or_default();
        contexts_by_project_id.insert(
            project.id,
            ProjectLineageContext {
                project_id: project.id,
                root_project_id,
                source_mode,
                snapshot: selected_snapshot,
                snapshot_chapters_by_source,
            },
        );
    }
    project_ids.sort_unstable();
    project_ids.dedup();

    Ok(FamilyLineageContext {
        project_ids,
        contexts_by_project_id,
    })
}
