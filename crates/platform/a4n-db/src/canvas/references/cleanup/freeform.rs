use super::super::config::validate_positive_id;
use super::super::reset::{
    ResetTargetLookup, clear_reset_generated_ref_payload_with_lookup,
    reset_missing_canvas_ref_for_fields_with_lookup,
};
use super::scope::canvas_nodes_for_projects;
use super::{MissingCanvasNodeUpdate, batch_update_missing_canvas_nodes};
use crate::canvas::CanvasDbError;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use serde_json::Value;

pub(crate) async fn mark_freeform_canvas_generated_refs_missing<C>(
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

    let lookup = ResetTargetLookup::from_targets(targets);
    let nodes = canvas_nodes_for_projects(db, std::slice::from_ref(&project_id)).await?;
    let now = Utc::now().naive_utc();
    let mut updates = Vec::new();
    for node in nodes {
        let Some(missing_ref) = reset_missing_canvas_ref_for_fields_with_lookup(
            node.ref_type.as_deref(),
            node.ref_id,
            &node.config_json,
            &lookup,
        ) else {
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
            clear_reset_generated_ref_payload_with_lookup(object, &lookup);
        }

        updates.push(MissingCanvasNodeUpdate {
            id: node.id,
            ref_type: (!missing_ref.clear_node_ref)
                .then(|| node.ref_type.clone())
                .flatten(),
            ref_id: (!missing_ref.clear_node_ref)
                .then_some(node.ref_id)
                .flatten(),
            config_json: config,
        });
    }
    batch_update_missing_canvas_nodes(db, updates, now).await
}
