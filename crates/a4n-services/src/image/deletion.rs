use a4n_core::error::AppError;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) async fn clear_generated_image_references(
    tx: &DatabaseTransaction,
    image_ids: &[i64],
) -> Result<(), AppError> {
    if image_ids.is_empty() {
        return Ok(());
    }

    let panels = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_in(image_ids.to_vec()))
        .all(tx)
        .await
        .map_err(AppError::Database)?;
    for panel in panels {
        let mut active: chapter_manga_prompt::ActiveModel = panel.into();
        active.generated_image_id = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(tx).await.map_err(AppError::Database)?;
    }

    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::SourceImageId.is_in(image_ids.to_vec()))
        .all(tx)
        .await
        .map_err(AppError::Database)?;
    for video in videos {
        let mut active: generated_video::ActiveModel = video.into();
        active.source_image_id = Set(None);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(tx).await.map_err(AppError::Database)?;
    }

    mark_canvas_image_refs_missing(tx, image_ids).await?;

    Ok(())
}

async fn mark_canvas_image_refs_missing(
    tx: &DatabaseTransaction,
    image_ids: &[i64],
) -> Result<(), AppError> {
    let image_ids = image_ids.to_vec();
    let project_ids = generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(image_ids.clone()))
        .all(tx)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|image| image.project_id)
        .collect::<BTreeSet<_>>();
    if project_ids.is_empty() {
        return Ok(());
    }
    let graph_ids = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.is_in(project_ids.into_iter().collect::<Vec<_>>()))
        .all(tx)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    if graph_ids.is_empty() {
        return Ok(());
    }
    let nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids))
        .all(tx)
        .await
        .map_err(AppError::Database)?;
    let now = chrono::Utc::now().naive_utc();
    for node in nodes {
        if !canvas_node_references_generated_image(&node, &image_ids) {
            continue;
        }
        let primary_ref_matches = node.ref_type.as_deref() == Some("legacy_generated_image")
            && node.ref_id.is_some_and(|id| image_ids.contains(&id));
        let missing_ref_id = node
            .ref_id
            .filter(|id| image_ids.contains(id))
            .or_else(|| first_config_i64(&node.config_json, IMAGE_REF_CONFIG_KEYS))
            .or_else(|| {
                first_config_array_i64(&node.config_json, "reference_image_ids", &image_ids)
            });
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated image was deleted".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String("generated_image".to_string()),
            );
            if let Some(id) = missing_ref_id {
                object.insert("missing_ref_id".to_string(), Value::from(id));
            }
            remove_config_i64_refs(object, IMAGE_REF_CONFIG_KEYS, &image_ids);
            remove_config_i64_array_refs(object, "reference_image_ids", &image_ids);
        }
        let mut active: canvas_node::ActiveModel = node.into();
        if primary_ref_matches {
            active.ref_type = Set(None);
            active.ref_id = Set(None);
        }
        active.state = Set("missing".to_string());
        active.config_json = Set(config);
        active.updated_at = Set(now);
        active.update(tx).await.map_err(AppError::Database)?;
    }
    Ok(())
}

const IMAGE_REF_CONFIG_KEYS: &[&str] = &[
    "generated_image_id",
    "legacy_generated_image_id",
    "source_image_id",
    "first_frame_image_id",
];

fn canvas_node_references_generated_image(node: &canvas_node::Model, image_ids: &[i64]) -> bool {
    if node.ref_type.as_deref() == Some("legacy_generated_image")
        && node.ref_id.is_some_and(|id| image_ids.contains(&id))
    {
        return true;
    }
    config_contains_any_i64(&node.config_json, IMAGE_REF_CONFIG_KEYS, image_ids)
        || config_array_contains_any_i64(&node.config_json, "reference_image_ids", image_ids)
}

fn config_contains_any_i64(config: &Value, keys: &[&str], ids: &[i64]) -> bool {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .any(|id| ids.contains(&id))
}

fn config_array_contains_any_i64(config: &Value, key: &str, ids: &[i64]) -> bool {
    config
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(Value::as_i64)
                .any(|id| ids.contains(&id))
        })
}

fn first_config_i64(config: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| config.get(*key).and_then(Value::as_i64))
}

fn first_config_array_i64(config: &Value, key: &str, ids: &[i64]) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .find(|id| ids.contains(id))
}

fn remove_config_i64_refs(object: &mut serde_json::Map<String, Value>, keys: &[&str], ids: &[i64]) {
    for key in keys {
        if object
            .get(*key)
            .and_then(Value::as_i64)
            .is_some_and(|id| ids.contains(&id))
        {
            object.remove(*key);
        }
    }
}

fn remove_config_i64_array_refs(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    ids: &[i64],
) {
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        return;
    };
    let retained = values
        .iter()
        .filter(|value| value.as_i64().is_none_or(|id| !ids.contains(&id)))
        .cloned()
        .collect::<Vec<_>>();
    if retained.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(retained));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_image_cleanup_removes_only_deleted_generated_image_refs() {
        let mut config = json!({
            "generated_image_id": 10,
            "source_image_id": 11,
            "reference_image_ids": [10, 12],
            "title": "still valid"
        });
        let object = config.as_object_mut().expect("object config");

        remove_config_i64_refs(object, IMAGE_REF_CONFIG_KEYS, &[10]);
        remove_config_i64_array_refs(object, "reference_image_ids", &[10]);

        assert!(config.get("generated_image_id").is_none());
        assert_eq!(config["source_image_id"], json!(11));
        assert_eq!(config["reference_image_ids"], json!([12]));
        assert_eq!(config["title"], json!("still valid"));
    }
}
