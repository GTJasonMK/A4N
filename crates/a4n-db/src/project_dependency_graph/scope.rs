use a4n_core::error::AppError;
use a4n_core::models::project::{novel_project, project_derivation};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct AssetGraphScope {
    pub(super) root_project: novel_project::Model,
    pub(super) family_project_ids: Vec<i64>,
    pub(super) derived_projects: Vec<novel_project::Model>,
    pub(super) project_derivations: Vec<project_derivation::Model>,
}

pub(super) async fn load_asset_graph_scope<C>(
    db: &C,
    project: novel_project::Model,
) -> Result<AssetGraphScope, AppError>
where
    C: ConnectionTrait,
{
    let requested_project_id = project.id;
    let root_project_id = project.root_project_id.unwrap_or(project.id);
    let root_project = if project.id == root_project_id {
        project
    } else {
        novel_project::Entity::find_by_id(root_project_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("根文本项目不存在: {root_project_id}")))?
    };
    let project_derivations = project_derivation::Entity::find()
        .filter(project_derivation::Column::RootProjectId.eq(root_project.id))
        .order_by_asc(project_derivation::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let derived_projects = novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(root_project.id))
        .order_by_asc(novel_project::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut family_project_ids = BTreeSet::from([root_project.id, requested_project_id]);
    family_project_ids.extend(
        project_derivations
            .iter()
            .map(|derivation| derivation.derived_project_id),
    );
    family_project_ids.extend(derived_projects.iter().map(|project| project.id));
    Ok(AssetGraphScope {
        root_project,
        family_project_ids: family_project_ids.into_iter().collect(),
        derived_projects,
        project_derivations,
    })
}
