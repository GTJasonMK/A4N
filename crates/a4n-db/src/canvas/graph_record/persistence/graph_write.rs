use a4n_core::models::canvas::canvas_graph;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::context::{
    CanvasProjectIdentity, default_canvas_record_graph_kind, normalize_canvas_record_graph_kind,
};
use crate::canvas::{CanvasDbError, CanvasGraphRecordInput};

pub(super) async fn upsert_canvas_graph_record<C>(
    db: &C,
    project: &CanvasProjectIdentity,
    project_id: i64,
    input: &CanvasGraphRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<canvas_graph::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    match input.id {
        Some(graph_id) => {
            let existing = canvas_graph::Entity::find_by_id(graph_id)
                .one(db)
                .await?
                .ok_or_else(|| CanvasDbError::NotFound(format!("画布不存在: {graph_id}")))?;
            if existing.project_id != project_id {
                return Err(CanvasDbError::Forbidden("画布不属于当前项目".to_string()));
            }
            let expected_version = input.expected_version.ok_or_else(|| {
                CanvasDbError::InvalidArgument("更新画布必须提供 expected_version".to_string())
            })?;
            if expected_version <= 0 {
                return Err(CanvasDbError::InvalidArgument(
                    "expected_version 必须大于 0".to_string(),
                ));
            }
            if existing.version != expected_version {
                return Err(CanvasDbError::VersionConflict {
                    graph_id,
                    expected_version,
                    current_version: existing.version,
                });
            }
            let next_version = expected_version.checked_add(1).ok_or_else(|| {
                CanvasDbError::InvalidArgument("画布 version 超出 i32 范围".to_string())
            })?;
            let name = input
                .name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&existing.name)
                .to_string();
            let graph_kind = match input.graph_kind.as_deref() {
                Some(kind) => normalize_canvas_record_graph_kind(kind)?.to_string(),
                None => existing.graph_kind,
            };
            let update = canvas_graph::Entity::update_many()
                .col_expr(canvas_graph::Column::Name, Expr::value(name))
                .col_expr(canvas_graph::Column::GraphKind, Expr::value(graph_kind))
                .col_expr(
                    canvas_graph::Column::ViewportJson,
                    Expr::value(input.viewport_json.clone()),
                )
                .col_expr(canvas_graph::Column::Version, Expr::value(next_version))
                .col_expr(canvas_graph::Column::UpdatedAt, Expr::value(now))
                .filter(canvas_graph::Column::Id.eq(graph_id))
                .filter(canvas_graph::Column::ProjectId.eq(project_id))
                .filter(canvas_graph::Column::Version.eq(expected_version))
                .exec(db)
                .await?;
            if update.rows_affected != 1 {
                let current_version = canvas_graph::Entity::find_by_id(graph_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("画布不存在: {graph_id}")))?
                    .version;
                return Err(CanvasDbError::VersionConflict {
                    graph_id,
                    expected_version,
                    current_version,
                });
            }
            canvas_graph::Entity::find_by_id(graph_id)
                .one(db)
                .await?
                .ok_or_else(|| CanvasDbError::NotFound(format!("画布不存在: {graph_id}")))
        }
        None => {
            if input.expected_version.is_some() {
                return Err(CanvasDbError::InvalidArgument(
                    "新建画布不能提供 expected_version".to_string(),
                ));
            }
            let graph_kind = match input.graph_kind.as_deref() {
                Some(value) => normalize_canvas_record_graph_kind(value)?,
                None => default_canvas_record_graph_kind(project)?,
            };
            canvas_graph::ActiveModel {
                project_id: Set(project_id),
                name: Set(input
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("视频画布")
                    .to_string()),
                graph_kind: Set(graph_kind.to_string()),
                version: Set(1),
                viewport_json: Set(input.viewport_json.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(CanvasDbError::Database)
        }
    }
}
