use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ConnectionTrait, EntityTrait};

use super::GRAPH_KIND_VIDEO_STUDIO;
use crate::canvas::{CanvasDbError, PROJECT_KIND_VIDEO};

const GRAPH_KIND_DERIVED_VIDEO: &str = "derived_video";
const GRAPH_KIND_TEMPLATE: &str = "template";

pub(super) async fn require_canvas_project_record<C>(
    db: &C,
    project_id: i64,
) -> Result<novel_project::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_VIDEO {
        return Err(CanvasDbError::InvalidArgument(
            "画布工作台当前只支持视频项目".to_string(),
        ));
    }
    project
        .source_mode()
        .map_err(|error| CanvasDbError::InvalidArgument(format!("项目 {project_id}: {error}")))?;
    Ok(project)
}

pub(super) fn normalize_canvas_record_graph_kind(
    value: &str,
) -> Result<&'static str, CanvasDbError> {
    match value.trim() {
        GRAPH_KIND_VIDEO_STUDIO => Ok(GRAPH_KIND_VIDEO_STUDIO),
        GRAPH_KIND_DERIVED_VIDEO => Ok(GRAPH_KIND_DERIVED_VIDEO),
        GRAPH_KIND_TEMPLATE => Ok(GRAPH_KIND_TEMPLATE),
        other => Err(CanvasDbError::InvalidArgument(format!(
            "未知画布类型: {other}"
        ))),
    }
}

pub(super) fn default_canvas_record_graph_kind(
    project: &novel_project::Model,
) -> Result<&'static str, CanvasDbError> {
    match project
        .source_mode()
        .map_err(|error| CanvasDbError::InvalidArgument(format!("项目 {}: {error}", project.id)))?
    {
        ProjectSourceMode::Derived => Ok(GRAPH_KIND_DERIVED_VIDEO),
        ProjectSourceMode::Freeform => Ok(GRAPH_KIND_VIDEO_STUDIO),
        ProjectSourceMode::Self_ => Err(CanvasDbError::InvalidArgument(
            "视频画布不支持文本项目".to_string(),
        )),
    }
}
