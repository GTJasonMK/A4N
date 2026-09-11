use super::super::config::{
    IMAGE_REF_CONFIG_KEY, config_id_in, remove_config_any_i64_ref, validate_positive_id,
};
use super::super::reset::canvas_node_references_generated_image_fields;
use super::scope::canvas_nodes_for_projects;
use super::{MissingCanvasNodeUpdate, batch_update_missing_canvas_nodes};
use crate::canvas::CanvasDbError;
use a4n_db_models::models::ai::generated_image;
use a4n_domain::CanvasNodeRefKind;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

pub(crate) async fn mark_canvas_generated_image_refs_missing<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(());
    }
    for image_id in image_ids {
        validate_positive_id(*image_id, "image_id")?;
    }

    let image_ids = image_ids.to_vec();
    let image_id_set = image_ids.iter().copied().collect::<HashSet<_>>();
    let project_ids = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::ProjectId)
        .filter(generated_image::Column::Id.is_in(image_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let nodes = canvas_nodes_for_projects(db, &project_ids).await?;
    let now = Utc::now().naive_utc();
    let mut updates = Vec::new();
    for node in nodes {
        if !canvas_node_references_generated_image_fields(
            node.ref_type.as_deref(),
            node.ref_id,
            &node.config_json,
            &image_id_set,
        ) {
            continue;
        }
        let primary_ref_matches = node.ref_type.as_deref()
            == Some(CanvasNodeRefKind::GeneratedImage.as_str())
            && node.ref_id.is_some_and(|id| image_id_set.contains(&id));
        let missing_ref_id = node
            .ref_id
            .filter(|id| image_id_set.contains(id))
            .or_else(|| {
                config_id_in(&node.config_json, IMAGE_REF_CONFIG_KEY, |id| {
                    image_id_set.contains(id)
                })
            });
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated image was deleted".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String(CanvasNodeRefKind::GeneratedImage.to_string()),
            );
            if let Some(id) = missing_ref_id {
                object.insert("missing_ref_id".to_string(), Value::from(id));
            }
            remove_config_any_i64_ref(object, IMAGE_REF_CONFIG_KEY, |id| image_id_set.contains(id));
        }

        updates.push(MissingCanvasNodeUpdate {
            id: node.id,
            ref_type: (!primary_ref_matches)
                .then(|| node.ref_type.clone())
                .flatten(),
            ref_id: (!primary_ref_matches).then_some(node.ref_id).flatten(),
            config_json: config,
        });
    }
    batch_update_missing_canvas_nodes(db, updates, now).await
}
