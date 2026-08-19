use super::super::config::{
    IMAGE_REF_CONFIG_KEY, config_id_in, remove_config_any_i64_ref, validate_positive_id,
};
use super::super::reset::canvas_node_references_generated_image;
use super::scope::canvas_nodes_for_projects;
use crate::canvas::CanvasDbError;
use a4n_core::models::{ai::generated_image, canvas::canvas_node};
use a4n_domain::CanvasNodeRefKind;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeSet;

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
    let project_ids = generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(image_ids.clone()))
        .all(db)
        .await?
        .into_iter()
        .map(|image| image.project_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let nodes = canvas_nodes_for_projects(db, &project_ids).await?;
    let now = Utc::now().naive_utc();
    for node in nodes {
        if !canvas_node_references_generated_image(&node, &image_ids) {
            continue;
        }
        let primary_ref_matches = node.ref_type.as_deref()
            == Some(CanvasNodeRefKind::GeneratedImage.as_str())
            && node.ref_id.is_some_and(|id| image_ids.contains(&id));
        let missing_ref_id = node
            .ref_id
            .filter(|id| image_ids.contains(id))
            .or_else(|| config_id_in(&node.config_json, IMAGE_REF_CONFIG_KEY, &image_ids));
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
            remove_config_any_i64_ref(object, IMAGE_REF_CONFIG_KEY, &image_ids);
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
