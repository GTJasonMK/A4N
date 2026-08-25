use super::super::config::{
    IMAGE_REF_CONFIG_KEY, VIDEO_REF_CONFIG_KEY, config_id_in, validate_positive_id,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};
use std::collections::{BTreeSet, HashMap, HashSet};

pub(crate) async fn generated_video_ref_project_ids<C>(
    db: &C,
    video_id: i64,
) -> Result<BTreeSet<i64>, CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(video_id, "video_id")?;

    let graphs = canvas_graph::Entity::find()
        .select_only()
        .column(canvas_graph::Column::Id)
        .column(canvas_graph::Column::ProjectId)
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();
    if graphs.is_empty() {
        return Ok(BTreeSet::new());
    }

    let project_ids = canvas_node::Entity::find()
        .select_only()
        .column(canvas_node::Column::GraphId)
        .column(canvas_node::Column::RefType)
        .column(canvas_node::Column::RefId)
        .column(canvas_node::Column::ConfigJson)
        .into_tuple::<(i64, Option<String>, Option<i64>, serde_json::Value)>()
        .all(db)
        .await?
        .into_iter()
        .filter(|(_, ref_type, ref_id, config_json)| {
            canvas_node_references_generated_video_fields(
                ref_type.as_deref(),
                *ref_id,
                config_json,
                video_id,
            )
        })
        .filter_map(|(graph_id, _, _, _)| graphs.get(&graph_id).copied())
        .collect::<BTreeSet<_>>();
    Ok(project_ids)
}

fn canvas_node_references_generated_video_fields(
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: &serde_json::Value,
    video_id: i64,
) -> bool {
    (ref_type == Some(CanvasNodeRefKind::GeneratedVideo.as_str()) && ref_id == Some(video_id))
        || config_id_in(config_json, VIDEO_REF_CONFIG_KEY, |id| *id == video_id).is_some()
}

#[cfg(test)]
pub(in crate::canvas::references) fn canvas_node_references_generated_video(
    node: &canvas_node::Model,
    video_id: i64,
) -> bool {
    canvas_node_references_generated_video_fields(
        node.ref_type.as_deref(),
        node.ref_id,
        &node.config_json,
        video_id,
    )
}

#[cfg(test)]
pub(in crate::canvas::references) fn canvas_node_references_generated_image(
    node: &canvas_node::Model,
    image_ids: &[i64],
) -> bool {
    let image_id_set = image_ids.iter().copied().collect::<HashSet<_>>();
    canvas_node_references_generated_image_fields(
        node.ref_type.as_deref(),
        node.ref_id,
        &node.config_json,
        &image_id_set,
    )
}

pub(in crate::canvas::references) fn canvas_node_references_generated_image_fields(
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: &serde_json::Value,
    image_ids: &HashSet<i64>,
) -> bool {
    (ref_type == Some(CanvasNodeRefKind::GeneratedImage.as_str())
        && ref_id.is_some_and(|id| image_ids.contains(&id)))
        || config_id_in(config_json, IMAGE_REF_CONFIG_KEY, |id| {
            image_ids.contains(id)
        })
        .is_some()
}
