use std::collections::HashMap;

use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_node,
    media::media_asset,
};
use a4n_domain::canvas::{
    CANVAS_MEDIA_INPUT_METADATA_KEYS, CanvasNodeRefKind, canvas_media_input_ref_kind,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::canvas::CanvasDbError;

pub(super) fn clear_media_input_metadata(
    node_kind: &str,
    config: &mut Value,
) -> Result<(), CanvasDbError> {
    if canvas_media_input_ref_kind(node_kind).is_none() {
        return Ok(());
    }
    let config = config.as_object_mut().ok_or_else(|| {
        CanvasDbError::InvalidArgument(format!("{node_kind} config_json 必须是 JSON 对象"))
    })?;
    for key in CANVAS_MEDIA_INPUT_METADATA_KEYS {
        config.remove(key);
    }
    Ok(())
}

pub(super) async fn populate_media_input_metadata<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    nodes: &mut [canvas_node::Model],
) -> Result<(), CanvasDbError> {
    let ids = |kind| {
        nodes
            .iter()
            .filter(|node| input_reference(node).is_some_and(|(ref_kind, _)| ref_kind == kind))
            .filter_map(|node| node.ref_id)
            .collect::<Vec<_>>()
    };
    let image_ids = ids(CanvasNodeRefKind::GeneratedImage);
    let video_ids = ids(CanvasNodeRefKind::GeneratedVideo);
    let audio_ids = ids(CanvasNodeRefKind::MediaAsset);
    let mut metadata = HashMap::new();
    if !image_ids.is_empty() {
        for image in generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::Id.is_in(image_ids))
            .all(db)
            .await?
        {
            metadata.insert(
                (CanvasNodeRefKind::GeneratedImage, image.id),
                json!({
                    "file_path": image.file_path,
                    "width": image.width,
                    "height": image.height,
                    "file_size": image.file_size,
                }),
            );
        }
    }
    if !video_ids.is_empty() {
        for video in generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::Id.is_in(video_ids))
            .all(db)
            .await?
        {
            metadata.insert(
                (CanvasNodeRefKind::GeneratedVideo, video.id),
                json!({
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "width": video.width,
                    "height": video.height,
                    "duration_secs": video.duration_secs,
                    "fps": video.fps,
                    "file_size": video.file_size,
                    "status": video.status,
                    "error_message": video.error_message,
                }),
            );
        }
    }
    if !audio_ids.is_empty() {
        for asset in media_asset::Entity::find()
            .filter(media_asset::Column::ProjectId.eq(project_id))
            .filter(media_asset::Column::Id.is_in(audio_ids))
            .all(db)
            .await?
        {
            metadata.insert(
                (CanvasNodeRefKind::MediaAsset, asset.id),
                json!({
                    "file_path": asset.file_path,
                    "thumbnail_path": asset.thumbnail_path,
                    "width": asset.width,
                    "height": asset.height,
                    "duration_secs": asset.duration_secs,
                    "fps": asset.fps,
                    "file_size": asset.file_size,
                    "status": asset.status,
                    "error_message": asset.error_message,
                }),
            );
        }
    }
    for node in nodes {
        clear_media_input_metadata(&node.node_kind, &mut node.config_json)?;
        let Some(reference) = input_reference(node) else {
            continue;
        };
        let resource = metadata.get(&reference).ok_or_else(|| {
            CanvasDbError::NotFound(format!(
                "画布输入引用不存在于当前项目: {} #{}",
                reference.0, reference.1
            ))
        })?;
        node.config_json
            .as_object_mut()
            .expect("media input config was validated above")
            .extend(resource.as_object().expect("metadata is an object").clone());
    }
    Ok(())
}

fn input_reference(node: &canvas_node::Model) -> Option<(CanvasNodeRefKind, i64)> {
    let kind = canvas_media_input_ref_kind(&node.node_kind)?;
    (node.ref_type.as_deref() == Some(kind.as_str()))
        .then_some(node.ref_id)
        .flatten()
        .map(|id| (kind, id))
}
