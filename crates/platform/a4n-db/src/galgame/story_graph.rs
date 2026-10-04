use a4n_db_models::models::galgame::galgame_story_graph;
use a4n_db_models::models::project::novel_project;
use a4n_domain::galgame::StoryGraph;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use super::types::GalgameStoryGraphRecordError;

#[derive(Debug, Clone)]
pub struct GalgameStoryGraphRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub revision: i64,
    pub status: String,
    pub graph: StoryGraph,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl GalgameStoryGraphRecord {
    pub fn is_user_edited(&self) -> bool {
        self.status == "edited"
    }
}

pub async fn current_galgame_story_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<GalgameStoryGraphRecord>, GalgameStoryGraphRecordError>
where
    C: ConnectionTrait,
{
    if project_id <= 0 {
        return Err(GalgameStoryGraphRecordError::InvalidArgument(
            "project_id 必须是正整数".into(),
        ));
    }
    let row = galgame_story_graph::Entity::find()
        .filter(galgame_story_graph::Column::ProjectId.eq(project_id))
        .order_by_desc(galgame_story_graph::Column::Revision)
        .one(db)
        .await?;
    row.map(map_record).transpose()
}

pub async fn delete_galgame_story_graph_revisions<C>(
    db: &C,
    project_id: i64,
) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    Ok(galgame_story_graph::Entity::delete_many()
        .filter(galgame_story_graph::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected)
}

pub async fn save_galgame_story_graph_revision<C>(
    db: &C,
    project_id: i64,
    expected_revision: i64,
    status: &str,
    graph: StoryGraph,
) -> Result<GalgameStoryGraphRecord, GalgameStoryGraphRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let record = save_galgame_story_graph_revision_in_transaction(
        &tx,
        project_id,
        expected_revision,
        status,
        graph,
    )
    .await?;
    tx.commit().await?;
    Ok(record)
}

pub(crate) async fn save_galgame_story_graph_revision_in_transaction<C>(
    db: &C,
    project_id: i64,
    expected_revision: i64,
    status: &str,
    mut graph: StoryGraph,
) -> Result<GalgameStoryGraphRecord, GalgameStoryGraphRecordError>
where
    C: ConnectionTrait,
{
    if project_id <= 0 || expected_revision < 0 || !matches!(status, "generated" | "edited") {
        return Err(GalgameStoryGraphRecordError::InvalidArgument(
            "project_id、expected_revision 或节点图状态无效".into(),
        ));
    }
    graph
        .validate()
        .map_err(GalgameStoryGraphRecordError::InvalidArgument)?;
    if graph.project_id != project_id {
        return Err(GalgameStoryGraphRecordError::InvalidArgument(
            "图项目与保存上下文不一致".into(),
        ));
    }

    let project = novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameStoryGraphRecordError::InvalidArgument(format!("项目不存在: {project_id}"))
        })?;
    if project.project_kind != "galgame"
        || project.root_project_id.is_none()
        || project.source_snapshot_id != Some(graph.source_snapshot_id)
    {
        return Err(GalgameStoryGraphRecordError::InvalidArgument(
            "节点图必须属于当前来源快照一致的 Galgame 项目".into(),
        ));
    }
    let current = current_galgame_story_graph(db, project_id).await?;
    let current_revision = current.as_ref().map_or(0, |record| record.revision);
    if current_revision != expected_revision {
        return Err(GalgameStoryGraphRecordError::RevisionConflict {
            expected_revision,
            current_revision,
        });
    }
    if current
        .as_ref()
        .is_some_and(|record| record.status == "stale")
    {
        return Err(GalgameStoryGraphRecordError::InvalidArgument(
            "来源已变化，过期节点图不能继续编辑".into(),
        ));
    }
    let next_revision = expected_revision.checked_add(1).ok_or_else(|| {
        GalgameStoryGraphRecordError::InvalidArgument("节点图 revision 超出范围".into())
    })?;
    let root_project_id = project.root_project_id.unwrap_or_default();
    graph.revision = next_revision;
    let now = Utc::now().naive_utc();
    let graph_json = serde_json::to_value(&graph).map_err(|error| {
        GalgameStoryGraphRecordError::InvalidArgument(format!("序列化节点图失败: {error}"))
    })?;
    let row = galgame_story_graph::ActiveModel {
        project_id: Set(project_id),
        root_project_id: Set(root_project_id),
        source_snapshot_id: Set(graph.source_snapshot_id),
        revision: Set(next_revision),
        status: Set(status.to_string()),
        graph_json: Set(graph_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    map_record(row)
}

fn map_record(
    row: galgame_story_graph::Model,
) -> Result<GalgameStoryGraphRecord, GalgameStoryGraphRecordError> {
    let graph: StoryGraph = serde_json::from_value(row.graph_json).map_err(|error| {
        GalgameStoryGraphRecordError::InvalidArgument(format!("节点图 JSON 无法解析: {error}"))
    })?;
    graph
        .validate()
        .map_err(GalgameStoryGraphRecordError::InvalidArgument)?;
    Ok(GalgameStoryGraphRecord {
        id: row.id,
        project_id: row.project_id,
        root_project_id: row.root_project_id,
        source_snapshot_id: row.source_snapshot_id,
        revision: row.revision,
        status: row.status,
        graph,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
