use super::super::canvas_node_references_generated_video;
use super::super::config::{
    VIDEO_REF_CONFIG_KEY, config_i64, remove_config_i64_ref, validate_positive_id,
};
use super::scope::canvas_nodes_for_project;
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::canvas_node;
use a4n_domain::CanvasNodeRefKind;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

pub async fn mark_canvas_generated_video_refs_missing<C>(
    db: &C,
    project_id: i64,
    video_id: i64,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    validate_positive_id(video_id, "video_id")?;

    let nodes = canvas_nodes_for_project(db, project_id).await?;
    let now = Utc::now().naive_utc();
    for node in nodes {
        if !canvas_node_references_generated_video(&node, video_id) {
            continue;
        }
        let primary_ref_matches = node.ref_type.as_deref()
            == Some(CanvasNodeRefKind::GeneratedVideo.as_str())
            && node.ref_id == Some(video_id);
        let missing_ref_id = node
            .ref_id
            .filter(|id| *id == video_id)
            .or_else(|| config_i64(&node.config_json, VIDEO_REF_CONFIG_KEY));
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated video was deleted".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String(CanvasNodeRefKind::GeneratedVideo.to_string()),
            );
            if let Some(id) = missing_ref_id {
                object.insert("missing_ref_id".to_string(), Value::from(id));
            }
            remove_config_i64_ref(object, VIDEO_REF_CONFIG_KEY, video_id);
        }

        let mut active: canvas_node::ActiveModel = node.into();
        if primary_ref_matches {
            active.ref_type = Set(None);
            active.ref_id = Set(None);
        }
        active.state = Set("missing".to_string());
        active.config_json = Set(config);
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    Ok(())
}
