use super::super::{SOURCE_MODE_DERIVED, validate_positive_i64};
use super::contracts::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError,
};
use super::projection::build_project_family_metadata;
use a4n_core::models::project::{novel_project, project_derivation, project_source_snapshot};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;

pub async fn project_family_metadata(
    db: &DatabaseConnection,
    query: ProjectFamilyMetadataQuery,
) -> Result<ProjectFamilyMetadata, ProjectFamilyMetadataReadError> {
    let mut metadata_by_project_id =
        project_family_metadata_batch(db, std::slice::from_ref(&query)).await?;
    metadata_by_project_id.remove(&query.project_id).ok_or(
        ProjectFamilyMetadataReadError::MetadataMissing {
            project_id: query.project_id,
        },
    )
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
    let mut derived_project_ids = Vec::new();
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
        if query.source_mode == SOURCE_MODE_DERIVED {
            derived_project_ids.push(query.project_id);
        }
    }
    dedup_i64(&mut root_project_ids);
    dedup_i64(&mut snapshot_ids);
    dedup_i64(&mut derived_project_ids);

    let roots_by_id = project_models_by_id(db, &root_project_ids).await?;
    let snapshots_by_id = source_snapshots_by_id(db, &snapshot_ids).await?;
    let derivations_by_project_id = derivations_by_project_id(db, &derived_project_ids).await?;

    let mut metadata_by_project_id = HashMap::with_capacity(queries.len());
    for query in queries {
        let metadata = build_project_family_metadata(
            query,
            &roots_by_id,
            &snapshots_by_id,
            &derivations_by_project_id,
        )?;
        metadata_by_project_id.insert(query.project_id, metadata);
    }
    Ok(metadata_by_project_id)
}

async fn project_models_by_id(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, novel_project::Model>, ProjectFamilyMetadataReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|error| ProjectFamilyMetadataReadError::database("查询项目族根项目失败", error))?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn source_snapshots_by_id(
    db: &DatabaseConnection,
    snapshot_ids: &[i64],
) -> Result<HashMap<i64, project_source_snapshot::Model>, ProjectFamilyMetadataReadError> {
    if snapshot_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = project_source_snapshot::Entity::find()
        .filter(project_source_snapshot::Column::Id.is_in(snapshot_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|error| {
            ProjectFamilyMetadataReadError::database("查询项目族源文本快照失败", error)
        })?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn derivations_by_project_id(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, project_derivation::Model>, ProjectFamilyMetadataReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|error| {
            ProjectFamilyMetadataReadError::database("查询项目族派生关系失败", error)
        })?;
    Ok(rows
        .into_iter()
        .map(|row| (row.derived_project_id, row))
        .collect())
}

fn dedup_i64(values: &mut Vec<i64>) {
    values.sort_unstable();
    values.dedup();
}
