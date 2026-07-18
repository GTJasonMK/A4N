use super::super::remap::{
    remap_video_studio_canvas_metadata, remap_video_studio_canvas_node_refs,
    remap_video_studio_required_id,
};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_node,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn update_imported_canvas_node_refs<C>(
    db: &C,
    rows: &VideoStudioProjectImportRows,
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for node in &rows.canvas_nodes {
        let node_id = remap_video_studio_required_id(&maps.node_ids, node.id, "canvas_node.id")?;
        let (ref_type, ref_id, config_json) = remap_video_studio_canvas_node_refs(
            node.ref_type.clone(),
            node.ref_id,
            node.config_json.clone(),
            maps,
        )?;
        canvas_node::ActiveModel {
            id: Set(node_id),
            ref_type: Set(ref_type),
            ref_id: Set(ref_id),
            config_json: Set(config_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn update_imported_media_metadata<C>(
    db: &C,
    rows: &VideoStudioProjectImportRows,
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for image in &rows.generated_images {
        let Some(new_id) = maps.image_ids.get(&image.id).copied() else {
            continue;
        };
        let metadata_json = image
            .metadata_json
            .clone()
            .map(|metadata| remap_video_studio_canvas_metadata(metadata, maps))
            .transpose()?;
        generated_image::ActiveModel {
            id: Set(new_id),
            metadata_json: Set(metadata_json),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    for video in &rows.generated_videos {
        let Some(new_id) = maps.video_ids.get(&video.id).copied() else {
            continue;
        };
        let metadata_json = video
            .metadata_json
            .clone()
            .map(|metadata| remap_video_studio_canvas_metadata(metadata, maps))
            .transpose()?;
        generated_video::ActiveModel {
            id: Set(new_id),
            metadata_json: Set(metadata_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
