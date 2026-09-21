use super::super::config::{
    VIDEO_REF_CONFIG_KEY, config_id_in, remove_config_any_i64_ref, validate_positive_id,
};
use super::scope::canvas_nodes_for_projects;
use super::{MissingCanvasNodeUpdate, batch_update_missing_canvas_nodes};
use crate::canvas::CanvasDbError;
use a4n_domain::CanvasNodeRefKind;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::HashSet;

pub(crate) async fn mark_canvas_generated_video_refs_missing_for_projects<C>(
    db: &C,
    project_ids: &[i64],
    video_id: i64,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    mark_canvas_generated_video_refs_missing_for_projects_and_videos(
        db,
        project_ids,
        std::slice::from_ref(&video_id),
    )
    .await
}

pub(crate) async fn mark_canvas_generated_video_refs_missing_for_projects_and_videos<C>(
    db: &C,
    project_ids: &[i64],
    video_ids: &[i64],
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() || video_ids.is_empty() {
        return Ok(());
    }
    for project_id in project_ids {
        validate_positive_id(*project_id, "project_id")?;
    }
    for video_id in video_ids {
        validate_positive_id(*video_id, "video_id")?;
    }
    let video_ids = video_ids.iter().copied().collect::<HashSet<_>>();

    let nodes = canvas_nodes_for_projects(db, project_ids).await?;
    let now = Utc::now().naive_utc();
    let mut updates = Vec::new();
    for node in nodes {
        let canonical_ref_id = (node.ref_type.as_deref()
            == Some(CanvasNodeRefKind::GeneratedVideo.as_str()))
        .then_some(node.ref_id)
        .flatten()
        .filter(|id| video_ids.contains(id));
        let config_ref_id = config_id_in(&node.config_json, VIDEO_REF_CONFIG_KEY, |id| {
            video_ids.contains(id)
        });
        if canonical_ref_id.is_none() && config_ref_id.is_none() {
            continue;
        }
        let primary_ref_matches = canonical_ref_id.is_some();
        let missing_ref_id = canonical_ref_id.or(config_ref_id);
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated video was deleted".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String(CanvasNodeRefKind::GeneratedVideo.to_string()),
            );
            if let Some(id) = missing_ref_id {
                object.insert("missing_ref_id".to_string(), Value::from(id));
            }
            remove_config_any_i64_ref(object, VIDEO_REF_CONFIG_KEY, |id| video_ids.contains(id));
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
