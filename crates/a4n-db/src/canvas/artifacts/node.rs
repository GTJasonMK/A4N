use super::WORKFLOW_STATUS_COMPLETED;
use super::types::{CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput};
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::canvas_node;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::Value;

pub async fn update_canvas_node_artifact_record<C>(
    db: &C,
    input: CanvasNodeArtifactUpdateInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let node = canvas_node::Entity::find_by_id(input.node_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!("画布节点不存在: {}", input.node_id))
        })?;
    let mut config_json = node.config_json;
    merge_canvas_artifact_config_patch(&mut config_json, input.config_patch);
    canvas_node::ActiveModel {
        id: Set(input.node_id),
        ref_type: Set(Some(input.ref_type)),
        ref_id: Set(Some(input.ref_id)),
        config_json: Set(config_json),
        state: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

pub async fn update_canvas_node_observation_record<C>(
    db: &C,
    input: CanvasNodeObservationUpdateInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let node = canvas_node::Entity::find_by_id(input.node_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!("画布节点不存在: {}", input.node_id))
        })?;
    let mut config_json = node.config_json;
    merge_canvas_observation_config_patch(&mut config_json, input.config_patch);
    canvas_node::ActiveModel {
        id: Set(input.node_id),
        config_json: Set(config_json),
        state: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

pub(super) fn merge_canvas_artifact_config_patch(config_json: &mut Value, config_patch: Value) {
    let (Value::Object(base), Value::Object(patch)) = (config_json, config_patch) else {
        return;
    };
    for (key, value) in patch {
        if value.is_null() {
            base.remove(&key);
        } else {
            base.insert(key, value);
        }
    }
}

pub(super) fn merge_canvas_observation_config_patch(config_json: &mut Value, config_patch: Value) {
    let (Value::Object(base), Value::Object(patch)) = (config_json, config_patch) else {
        return;
    };
    for (key, value) in patch {
        if !value.is_null() {
            base.insert(key, value);
        }
    }
}
