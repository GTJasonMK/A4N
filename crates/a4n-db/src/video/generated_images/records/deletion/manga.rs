use super::super::super::references::{
    should_delete_manga_page_image_fields, should_delete_manga_page_image_record,
};
use super::locking::generated_images_for_delete;
use super::references::clear_generated_image_references_record;
use crate::video::VideoDbError;
use a4n_core::models::ai::generated_image;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};

async fn delete_manga_page_image_records_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    preserve_image_id: Option<i64>,
) -> Result<Vec<String>, VideoDbError>
where
    C: ConnectionTrait,
{
    let panel_id = format!("page{page_number}");
    let candidate_images = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::PanelId)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .into_tuple::<(i64, String, Option<String>, Option<serde_json::Value>)>()
        .all(db)
        .await?
        .into_iter()
        .filter(|(image_id, image_type, image_panel_id, metadata)| {
            should_delete_manga_page_image_fields(
                *image_id,
                image_type,
                image_panel_id.as_deref(),
                metadata.as_ref(),
                &panel_id,
                page_number,
                preserve_image_id,
            )
        })
        .map(|(image_id, ..)| image_id)
        .collect::<Vec<_>>();
    let images = generated_images_for_delete(db, project_id, &candidate_images)
        .await?
        .into_iter()
        .filter(|image| {
            should_delete_manga_page_image_record(image, &panel_id, page_number, preserve_image_id)
        })
        .collect::<Vec<_>>();
    let file_paths = images
        .iter()
        .map(|image| image.file_path.clone())
        .collect::<Vec<_>>();
    let image_ids = images.iter().map(|image| image.id).collect::<Vec<_>>();
    if !image_ids.is_empty() {
        clear_generated_image_references_record(db, &image_ids).await?;
        crate::media::delete_generated_image_target_media_bindings(db, &image_ids).await?;
        crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids)
            .await?;
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::Id.is_in(image_ids))
            .exec(db)
            .await?;
    }
    Ok(file_paths)
}

pub async fn delete_manga_page_image_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    preserve_image_id: Option<i64>,
) -> Result<Vec<String>, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let file_paths = delete_manga_page_image_records_in(
        &txn,
        project_id,
        chapter_id,
        page_number,
        preserve_image_id,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(file_paths)
}
