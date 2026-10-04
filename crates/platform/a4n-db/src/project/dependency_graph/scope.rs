use a4n_db_models::models::project::novel_project;
use a4n_error::AppError;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::collections::BTreeSet;

#[derive(Debug, FromQueryResult)]
pub(super) struct AssetGraphProjectRow {
    pub(super) id: i64,
    pub(super) root_project_id: Option<i64>,
    pub(super) title: String,
    pub(super) project_kind: String,
    pub(super) project_type: String,
    pub(super) status: String,
    pub(super) target_chapters: i32,
    pub(super) current_chapter: i32,
}

#[derive(Debug, FromQueryResult)]
pub(super) struct AssetGraphDerivedProjectRow {
    pub(super) id: i64,
    pub(super) project_kind: String,
}

#[derive(Debug)]
pub(super) struct AssetGraphScope {
    pub(super) root_project: AssetGraphProjectRow,
    pub(super) family_project_ids: Vec<i64>,
    pub(super) derived_projects: Vec<AssetGraphDerivedProjectRow>,
}

pub(super) async fn load_asset_graph_scope<C>(
    db: &C,
    project_id: i64,
) -> Result<AssetGraphScope, AppError>
where
    C: ConnectionTrait,
{
    let project = asset_graph_project_query(project_id)
        .into_model::<AssetGraphProjectRow>()
        .one(db)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let requested_project_id = project.id;
    let root_project_id = project.root_project_id.unwrap_or(project.id);
    let root_project = if project.id == root_project_id {
        project
    } else {
        asset_graph_project_query(root_project_id)
            .into_model::<AssetGraphProjectRow>()
            .one(db)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::NotFound(format!("根文本项目不存在: {root_project_id}")))?
    };
    let derived_projects = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .filter(novel_project::Column::RootProjectId.eq(root_project.id))
        .order_by_asc(novel_project::Column::Id)
        .into_model::<AssetGraphDerivedProjectRow>()
        .all(db)
        .await
        .map_err(AppError::from)?;
    let mut family_project_ids = BTreeSet::from([root_project.id, requested_project_id]);
    family_project_ids.extend(derived_projects.iter().map(|project| project.id));
    Ok(AssetGraphScope {
        root_project,
        family_project_ids: family_project_ids.into_iter().collect(),
        derived_projects,
    })
}

fn asset_graph_project_query(project_id: i64) -> sea_orm::Select<novel_project::Entity> {
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::Title)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::ProjectType)
        .column(novel_project::Column::Status)
        .column(novel_project::Column::TargetChapters)
        .column(novel_project::Column::CurrentChapter)
}
