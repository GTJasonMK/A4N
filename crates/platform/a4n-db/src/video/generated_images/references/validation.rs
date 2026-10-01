use super::context::video_asset_project_ids_record;
use crate::profile_material::is_project_reusable_video_material_image;
use crate::video::VideoDbError;
use a4n_domain::image::reference_image_ids;
use a4n_domain::media::GeneratedImageRecord;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::video::media_mapping::generated_image_record_from_model;

pub async fn ensure_video_reference_images_available_record<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    validated_video_reference_image_records(db, project_id, chapter_id, reference_image_ids_json)
        .await
        .map(|_| ())
}

pub async fn validated_video_reference_image_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<Vec<GeneratedImageRecord>, VideoDbError>
where
    C: ConnectionTrait,
{
    let ids =
        reference_image_ids(reference_image_ids_json).map_err(VideoDbError::InvalidArgument)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    validated_video_reference_image_records_for_ids(db, project_id, chapter_id, vec![ids])
        .await
        .map(|mut images| images.pop().unwrap_or_default())
}

pub(crate) async fn validated_video_reference_image_records_batch<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: &[Option<&Value>],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let ids_by_prompt = reference_image_ids_json
        .iter()
        .map(|value| reference_image_ids(*value).map_err(VideoDbError::InvalidArgument))
        .collect::<Result<Vec<_>, _>>()?;
    validated_video_reference_image_records_for_ids(db, project_id, chapter_id, ids_by_prompt)
        .await
        .map(|_| ())
}

async fn validated_video_reference_image_records_for_ids<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    ids_by_prompt: Vec<Vec<i64>>,
) -> Result<Vec<Vec<GeneratedImageRecord>>, VideoDbError>
where
    C: ConnectionTrait,
{
    let all_ids = ids_by_prompt
        .iter()
        .flat_map(|ids| ids.iter().copied())
        .collect::<HashSet<_>>();
    if all_ids.is_empty() {
        return Ok(vec![Vec::new(); ids_by_prompt.len()]);
    }

    let allowed_project_ids = video_asset_project_ids_record(db, project_id)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    let rows = super::super::records::generated_image_records_by_ids(
        db,
        &all_ids.into_iter().collect::<Vec<_>>(),
    )
    .await?;
    let images_by_id = rows
        .into_iter()
        .map(generated_image_record_from_model)
        .map(|image| (image.id, image))
        .collect::<HashMap<_, _>>();

    ids_by_prompt
        .into_iter()
        .map(|ids| {
            ids.into_iter()
                .map(|id| {
                    let image = images_by_id
                        .get(&id)
                        .ok_or_else(|| VideoDbError::NotFound(format!("参考图片不存在: {id}")))?;
                    if !allowed_project_ids.contains(&image.project_id) {
                        return Err(VideoDbError::InvalidArgument(format!(
                            "参考图片 #{id} 不属于视频项目 {project_id} 或其根文本项目"
                        )));
                    }
                    if is_project_reusable_video_material_image(
                        &image.image_type,
                        image.metadata_json.as_ref(),
                    ) {
                        return Ok(image.clone());
                    }
                    if image.chapter_id.is_some() && image.chapter_id != Some(chapter_id) {
                        return Err(VideoDbError::InvalidArgument(format!(
                            "参考图片 #{id} 不属于提示词章节 {chapter_id}"
                        )));
                    }
                    Ok(image.clone())
                })
                .collect()
        })
        .collect()
}
