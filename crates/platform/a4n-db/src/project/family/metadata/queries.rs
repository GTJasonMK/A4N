use super::super::super::validate_positive_i64;
use super::contracts::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError,
    ProjectFamilyRootRow, ProjectFamilySnapshotRow,
};
use super::projection::build_project_family_metadata;
use a4n_db_models::models::project::{novel_project, project_source_snapshot};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

pub async fn project_family_metadata(
    db: &DatabaseConnection,
    query: ProjectFamilyMetadataQuery,
) -> Result<ProjectFamilyMetadata, ProjectFamilyMetadataReadError> {
    let mut metadata_by_project_id =
        project_family_metadata_batch(db, std::slice::from_ref(&query)).await?;
    Ok(metadata_by_project_id
        .remove(&query.project_id)
        .expect("单项族元数据查询必须产生对应项目结果"))
}

pub async fn project_family_metadata_batch(
    db: &DatabaseConnection,
    queries: &[ProjectFamilyMetadataQuery],
) -> Result<HashMap<i64, ProjectFamilyMetadata>, ProjectFamilyMetadataReadError> {
    if queries.is_empty() {
        return Ok(HashMap::new());
    }

    let mut root_project_ids = Vec::new();
    let mut snapshot_ids = Vec::new();
    for query in queries {
        validate_positive_i64(query.project_id, "project_id")
            .map_err(ProjectFamilyMetadataReadError::InvalidArgument)?;
        if let Some(root_project_id) = query.root_project_id {
            validate_positive_i64(root_project_id, "root_project_id")
                .map_err(ProjectFamilyMetadataReadError::InvalidArgument)?;
            root_project_ids.push(root_project_id);
        }
        if let Some(source_snapshot_id) = query.source_snapshot_id {
            validate_positive_i64(source_snapshot_id, "source_snapshot_id")
                .map_err(ProjectFamilyMetadataReadError::InvalidArgument)?;
            snapshot_ids.push(source_snapshot_id);
        }
    }
    dedup_i64(&mut root_project_ids);
    dedup_i64(&mut snapshot_ids);

    let (roots_by_id, snapshots_by_id) = futures::join!(
        project_models_by_id(db, &root_project_ids),
        source_snapshots_by_id(db, &snapshot_ids),
    );
    let roots_by_id = roots_by_id?;
    let snapshots_by_id = snapshots_by_id?;

    let mut metadata_by_project_id = HashMap::with_capacity(queries.len());
    for query in queries {
        let metadata = build_project_family_metadata(query, &roots_by_id, &snapshots_by_id)?;
        metadata_by_project_id.insert(query.project_id, metadata);
    }
    Ok(metadata_by_project_id)
}

async fn project_models_by_id(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectFamilyRootRow>, ProjectFamilyMetadataReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::Title)
        .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
        .into_model::<ProjectFamilyRootRow>()
        .all(db)
        .await
        .map_err(|error| ProjectFamilyMetadataReadError::database("查询项目族根项目失败", error))?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn source_snapshots_by_id(
    db: &DatabaseConnection,
    snapshot_ids: &[i64],
) -> Result<HashMap<i64, ProjectFamilySnapshotRow>, ProjectFamilyMetadataReadError> {
    if snapshot_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = project_source_snapshot::Entity::find()
        .select_only()
        .column(project_source_snapshot::Column::Id)
        .column(project_source_snapshot::Column::ChapterScopeJson)
        .filter(project_source_snapshot::Column::Id.is_in(snapshot_ids.iter().copied()))
        .into_model::<ProjectFamilySnapshotRow>()
        .all(db)
        .await
        .map_err(|error| {
            ProjectFamilyMetadataReadError::database("查询项目族源文本快照失败", error)
        })?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

fn dedup_i64(values: &mut Vec<i64>) {
    values.sort_unstable();
    values.dedup();
}
