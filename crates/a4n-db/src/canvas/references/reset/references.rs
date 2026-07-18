use super::super::config::{
    IMAGE_REF_CONFIG_KEYS, VIDEO_REF_CONFIG_KEYS, config_array_contains_any_i64,
    config_array_contains_i64, config_contains_any_i64, config_contains_i64, validate_positive_id,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use sea_orm::{ConnectionTrait, EntityTrait};
use std::collections::{BTreeMap, BTreeSet};

pub async fn generated_video_ref_project_ids<C>(
    db: &C,
    video_id: i64,
) -> Result<BTreeSet<i64>, CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(video_id, "video_id")?;

    let graphs = canvas_graph::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|graph| (graph.id, graph.project_id))
        .collect::<BTreeMap<_, _>>();
    if graphs.is_empty() {
        return Ok(BTreeSet::new());
    }

    let project_ids = canvas_node::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .filter(|node| canvas_node_references_generated_video(node, video_id))
        .filter_map(|node| graphs.get(&node.graph_id).copied())
        .collect::<BTreeSet<_>>();
    Ok(project_ids)
}

pub fn canvas_node_references_generated_video(node: &canvas_node::Model, video_id: i64) -> bool {
    if node.ref_type.as_deref() == Some("legacy_generated_video") && node.ref_id == Some(video_id) {
        return true;
    }
    config_contains_i64(&node.config_json, VIDEO_REF_CONFIG_KEYS, video_id)
        || config_array_contains_i64(&node.config_json, "video_ids", video_id)
}

pub fn canvas_node_references_generated_image(
    node: &canvas_node::Model,
    image_ids: &[i64],
) -> bool {
    if node.ref_type.as_deref() == Some("legacy_generated_image")
        && node.ref_id.is_some_and(|id| image_ids.contains(&id))
    {
        return true;
    }
    config_contains_any_i64(&node.config_json, IMAGE_REF_CONFIG_KEYS, image_ids)
        || config_array_contains_any_i64(&node.config_json, "reference_image_ids", image_ids)
}
