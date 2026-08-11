mod build;
mod detail;
mod records;

use a4n_core::{error::AppError, models::project::novel_project};
use sea_orm::{ConnectionTrait, EntityTrait};

use self::{build::build_asset_dependency_graph, records::AssetGraphRecords};
use super::{AssetDependencyGraph, load_asset_graph_scope, scope::AssetGraphScope};

pub use self::detail::asset_dependency_graph_node_detail;

/// 资产依赖图（项目族范围）。
pub async fn asset_dependency_graph(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<AssetDependencyGraph, AppError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let scope = load_asset_graph_scope(db, project).await?;

    asset_dependency_graph_for_scope(db, &scope).await
}

pub(super) async fn asset_dependency_graph_for_scope(
    db: &impl ConnectionTrait,
    scope: &AssetGraphScope,
) -> Result<AssetDependencyGraph, AppError> {
    let records =
        AssetGraphRecords::load(db, scope.root_project.id, &scope.family_project_ids).await?;
    build_asset_dependency_graph(scope, &records)
}
