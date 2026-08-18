use super::super::config::{
    IMAGE_REF_CONFIG_KEY, VIDEO_REF_CONFIG_KEY, config_id_in, validate_positive_id,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use a4n_domain::CanvasNodeRefKind;
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
    (node.ref_type.as_deref() == Some(CanvasNodeRefKind::GeneratedVideo.as_str())
        && node.ref_id == Some(video_id))
        || config_id_in(&node.config_json, VIDEO_REF_CONFIG_KEY, &[video_id]).is_some()
}

pub fn canvas_node_references_generated_image(
    node: &canvas_node::Model,
    image_ids: &[i64],
) -> bool {
    (node.ref_type.as_deref() == Some(CanvasNodeRefKind::GeneratedImage.as_str())
        && node.ref_id.is_some_and(|id| image_ids.contains(&id)))
        || config_id_in(&node.config_json, IMAGE_REF_CONFIG_KEY, image_ids).is_some()
}
