use super::super::remap::{
    remap_video_studio_canvas_metadata, remap_video_studio_canvas_node_refs,
    remap_video_studio_required_id,
};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_node,
};
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(crate) async fn update_imported_canvas_node_refs<C>(
    db: &C,
    rows: &VideoStudioProjectImportRows,
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    let mut ref_type_case = CaseStatement::new();
    let mut ref_id_case = CaseStatement::new();
    let mut config_json_case = CaseStatement::new();
    let mut node_ids = Vec::with_capacity(rows.canvas_nodes.len());
    for node in &rows.canvas_nodes {
        let node_id = remap_video_studio_required_id(&maps.node_ids, node.id, "canvas_node.id")?;
        let (ref_type, ref_id, config_json) = remap_video_studio_canvas_node_refs(
            node.ref_type.clone(),
            node.ref_id,
            node.config_json.clone(),
            maps,
        )?;
        ref_type_case =
            ref_type_case.case(canvas_node::Column::Id.eq(node_id), Expr::value(ref_type));
        ref_id_case = ref_id_case.case(canvas_node::Column::Id.eq(node_id), Expr::value(ref_id));
        config_json_case = config_json_case.case(
            canvas_node::Column::Id.eq(node_id),
            Expr::value(config_json).cast_as(Alias::new("json")),
        );
        node_ids.push(node_id);
    }
    if !node_ids.is_empty() {
        let updated = canvas_node::Entity::update_many()
            .col_expr(
                canvas_node::Column::RefType,
                ref_type_case
                    .finally(Expr::col(canvas_node::Column::RefType))
                    .into(),
            )
            .col_expr(
                canvas_node::Column::RefId,
                ref_id_case
                    .finally(Expr::col(canvas_node::Column::RefId))
                    .into(),
            )
            .col_expr(
                canvas_node::Column::ConfigJson,
                config_json_case
                    .finally(Expr::col(canvas_node::Column::ConfigJson))
                    .into(),
            )
            .col_expr(canvas_node::Column::UpdatedAt, Expr::value(now))
            .filter(canvas_node::Column::Id.is_in(node_ids.clone()))
            .exec(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if updated.rows_affected != node_ids.len() as u64 {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "导入画布节点引用更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                node_ids.len()
            )));
        }
    }
    Ok(())
}

pub(crate) async fn update_imported_media_metadata<C>(
    db: &C,
    rows: &VideoStudioProjectImportRows,
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    let mut image_metadata_case = CaseStatement::new();
    let mut image_ids = Vec::with_capacity(rows.generated_images.len());
    for image in &rows.generated_images {
        let Some(new_id) = maps.image_ids.get(&image.id).copied() else {
            continue;
        };
        let metadata_json = image
            .metadata_json
            .clone()
            .map(|metadata| remap_video_studio_canvas_metadata(metadata, maps))
            .transpose()?;
        image_metadata_case = image_metadata_case.case(
            generated_image::Column::Id.eq(new_id),
            Expr::value(metadata_json).cast_as(Alias::new("json")),
        );
        image_ids.push(new_id);
    }
    if !image_ids.is_empty() {
        let updated = generated_image::Entity::update_many()
            .col_expr(
                generated_image::Column::MetadataJson,
                image_metadata_case
                    .finally(Expr::col(generated_image::Column::MetadataJson))
                    .into(),
            )
            .filter(generated_image::Column::Id.is_in(image_ids.clone()))
            .exec(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if updated.rows_affected != image_ids.len() as u64 {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "导入生成图片元数据更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                image_ids.len()
            )));
        }
    }

    let mut video_metadata_case = CaseStatement::new();
    let mut video_ids = Vec::with_capacity(rows.generated_videos.len());
    for video in &rows.generated_videos {
        let Some(new_id) = maps.video_ids.get(&video.id).copied() else {
            continue;
        };
        let metadata_json = video
            .metadata_json
            .clone()
            .map(|metadata| remap_video_studio_canvas_metadata(metadata, maps))
            .transpose()?;
        video_metadata_case = video_metadata_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(metadata_json).cast_as(Alias::new("json")),
        );
        video_ids.push(new_id);
    }
    if !video_ids.is_empty() {
        let updated = generated_video::Entity::update_many()
            .col_expr(
                generated_video::Column::MetadataJson,
                video_metadata_case
                    .finally(Expr::col(generated_video::Column::MetadataJson))
                    .into(),
            )
            .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
            .filter(generated_video::Column::Id.is_in(video_ids.clone()))
            .exec(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if updated.rows_affected != video_ids.len() as u64 {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "导入生成视频元数据更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                video_ids.len()
            )));
        }
    }
    Ok(())
}
