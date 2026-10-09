use super::super::super::references::should_delete_manga_page_image_fields;
use super::locking::generated_images_for_delete;
use super::references::clear_generated_image_references_record;
use crate::video::VideoDbError;
use a4n_db_models::models::{ai::generated_image, chapter::chapter_manga_prompt};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};
use std::collections::HashSet;

pub(crate) async fn delete_manga_page_image_records_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    preserve_image_id: Option<i64>,
) -> Result<Vec<String>, VideoDbError>
where
    C: ConnectionTrait,
{
    let candidate_images =
        manga_page_image_ids_for_deletion(db, project_id, chapter_id, page_number).await?;
    delete_manga_image_records_in(db, project_id, &candidate_images, preserve_image_id).await
}

pub(crate) async fn manga_page_image_ids_for_deletion<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
) -> Result<Vec<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    let panel_id = format!("page{page_number}");
    let linked_image_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::GeneratedImageId)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .into_tuple::<Option<i64>>()
        .all(db)
        .await?
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    let mut query = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::PanelId)
        .column(generated_image::Column::MetadataJson)
        .column(generated_image::Column::ChapterId)
        .filter(generated_image::Column::ProjectId.eq(project_id));
    query = if linked_image_ids.is_empty() {
        query
            .filter(generated_image::Column::ChapterId.eq(chapter_id))
            .filter(generated_image::Column::ImageType.eq("manga_page"))
    } else {
        query.filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(generated_image::Column::ChapterId.eq(chapter_id))
                        .add(generated_image::Column::ImageType.eq("manga_page")),
                )
                .add(generated_image::Column::Id.is_in(linked_image_ids.iter().copied())),
        )
    };
    let candidate_images = query
        .into_tuple::<(
            i64,
            String,
            Option<String>,
            Option<serde_json::Value>,
            Option<i64>,
        )>()
        .all(db)
        .await?;
    let mut image_ids = candidate_images
        .into_iter()
        .filter(
            |(image_id, image_type, image_panel_id, metadata, image_chapter_id)| {
                image_type == "manga_page"
                    && (linked_image_ids.contains(image_id)
                        || (*image_chapter_id == Some(chapter_id)
                            && should_delete_manga_page_image_fields(
                                *image_id,
                                image_type,
                                image_panel_id.as_deref(),
                                metadata.as_ref(),
                                &panel_id,
                                page_number,
                                None,
                            )))
            },
        )
        .map(|(image_id, ..)| image_id)
        .collect::<Vec<_>>();
    image_ids.sort_unstable();
    image_ids.dedup();
    Ok(image_ids)
}

pub(crate) async fn delete_manga_chapter_image_records_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    preserve_image_id: Option<i64>,
) -> Result<Vec<String>, VideoDbError>
where
    C: ConnectionTrait,
{
    let candidate_images = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .filter(generated_image::Column::ImageType.eq("manga_page"))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    delete_manga_image_records_in(db, project_id, &candidate_images, preserve_image_id).await
}

async fn delete_manga_image_records_in<C>(
    db: &C,
    project_id: i64,
    candidate_images: &[i64],
    preserve_image_id: Option<i64>,
) -> Result<Vec<String>, VideoDbError>
where
    C: ConnectionTrait,
{
    let images = generated_images_for_delete(db, project_id, candidate_images)
        .await?
        .into_iter()
        .filter(|image| Some(image.id) != preserve_image_id)
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
