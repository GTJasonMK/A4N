use super::WORKFLOW_STATUS_COMPLETED;
use super::types::{CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput};
use crate::canvas::CanvasDbError;
use a4n_db_models::models::canvas::{canvas_graph, canvas_node};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set, TransactionTrait};
use serde_json::Value;

pub async fn update_canvas_node_artifact_record<C>(
    db: &C,
    input: CanvasNodeArtifactUpdateInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let node =
        canvas_node_for_graph_version(&tx, input.node_id, input.expected_graph_version).await?;
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
    .update(&tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_canvas_node_observation_record<C>(
    db: &C,
    input: CanvasNodeObservationUpdateInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let node =
        canvas_node_for_graph_version(&tx, input.node_id, input.expected_graph_version).await?;
    let mut config_json = node.config_json;
    merge_canvas_observation_config_patch(&mut config_json, input.config_patch);
    canvas_node::ActiveModel {
        id: Set(input.node_id),
        config_json: Set(config_json),
        state: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(&tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn canvas_node_for_graph_version<C>(
    tx: &C,
    node_id: i64,
    expected_graph_version: i32,
) -> Result<canvas_node::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    if expected_graph_version <= 0 {
        return Err(CanvasDbError::InvalidArgument(
            "Canvas 节点结果必须提供正数 graph_version".to_string(),
        ));
    }
    let node = canvas_node::Entity::find_by_id(node_id)
        .one(tx)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("画布节点不存在: {node_id}")))?;
    let graph = canvas_graph::Entity::find_by_id(node.graph_id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("画布不存在: {}", node.graph_id)))?;
    if graph.version != expected_graph_version {
        return Err(CanvasDbError::VersionConflict {
            graph_id: graph.id,
            expected_version: expected_graph_version,
            current_version: graph.version,
        });
    }
    Ok(node)
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
