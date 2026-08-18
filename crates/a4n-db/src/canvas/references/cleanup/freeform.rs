use super::super::config::validate_positive_id;
use super::super::{clear_reset_generated_ref_payload, reset_missing_canvas_ref_for_node};
use super::scope::canvas_nodes_for_projects;
use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::canvas_node;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

pub async fn mark_freeform_canvas_generated_refs_missing<C>(
    db: &C,
    project_id: i64,
    targets: &crate::project::FreeformGeneratedResetTargets,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    if targets.image_ids.is_empty()
        && targets.video_ids.is_empty()
        && targets.media_asset_ids.is_empty()
        && targets.generation_spec_ids.is_empty()
        && targets.prompt_artifact_ids.is_empty()
    {
        return Ok(());
    }

    let nodes = canvas_nodes_for_projects(db, std::slice::from_ref(&project_id)).await?;
    let now = Utc::now().naive_utc();
    for node in nodes {
        let Some(missing_ref) = reset_missing_canvas_ref_for_node(&node, targets) else {
            continue;
        };
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated freeform asset was reset".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String(missing_ref.ref_type.to_string()),
            );
            object.insert(
                "missing_ref_id".to_string(),
                Value::from(missing_ref.ref_id),
            );
            clear_reset_generated_ref_payload(object, targets);
        }

        let mut active: canvas_node::ActiveModel = node.into();
        if missing_ref.clear_node_ref {
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
