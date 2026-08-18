use a4n_core::models::{canvas::canvas_graph, project::novel_project};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use super::context::{default_canvas_record_graph_kind, normalize_canvas_record_graph_kind};
use crate::canvas::{CanvasDbError, CanvasGraphRecordInput};

pub(super) async fn upsert_canvas_graph_record<C>(
    db: &C,
    project: &novel_project::Model,
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
            let next_version = existing.version + 1;
            let mut active: canvas_graph::ActiveModel = existing.into();
            if let Some(name) = input
                .name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                active.name = Set(name.to_string());
            }
            if let Some(kind) = input.graph_kind.as_deref() {
                active.graph_kind = Set(normalize_canvas_record_graph_kind(kind)?.to_string());
            }
            active.viewport_json = Set(input.viewport_json.clone());
            active.version = Set(next_version);
            active.updated_at = Set(now);
            active.update(db).await.map_err(CanvasDbError::Database)
        }
        None => {
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
