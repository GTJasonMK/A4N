mod build;
mod detail;
mod records;

use a4n_core::error::AppError;
use sea_orm::ConnectionTrait;

use self::{build::build_asset_dependency_graph, records::AssetGraphRecords};
use super::{AssetDependencyGraph, load_asset_graph_scope, scope::AssetGraphScope};

pub use self::detail::asset_dependency_graph_node_detail;
pub(super) use self::records::{
    BlueprintGraphRow, BlueprintVersionGraphRow, ChapterGraphRow, ChapterOutlineGraphRow,
    CharacterProfileGraphRow, ConversationGraphRow, GeneratedImageGraphRow, GeneratedVideoGraphRow,
    GenerationSpecGraphRow, MangaPromptGraphRow, MediaAssetGraphRow, ObjectProfileGraphRow,
    PartOutlineGraphRow, PortraitGraphRow, PromptArtifactGraphRow, SceneProfileGraphRow,
    ShootingScriptGraphRow, VideoPromptGraphRow, VisualPlanGraphRow,
};

/// 资产依赖图（项目族范围）。
pub async fn asset_dependency_graph(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<AssetDependencyGraph, AppError> {
    let scope = load_asset_graph_scope(db, project_id).await?;

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
