use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    VideoPromptSourceRecord, generated_image_record, lineage_ref, video_prompt_sources_from_record,
};
use crate::project::dependency_graph::helpers::video_frame_image_prompt_id;
use a4n_db_models::models::{ai::generated_image, chapter::chapter_video_prompt};
use a4n_domain::image::{
    IMAGE_TYPE_VIDEO_KEY_FRAME, IMAGE_TYPE_VIDEO_START_FRAME, IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
};
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;

pub(in crate::project::dependency_graph) async fn video_frame_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let (prompts_result, images_result) = futures::join!(
        chapter_video_prompt::Entity::find()
            .select_only()
            .column(chapter_video_prompt::Column::Id)
            .column(chapter_video_prompt::Column::VisualPrompt)
            .column(chapter_video_prompt::Column::MotionPrompt)
            .column(chapter_video_prompt::Column::NegativePrompt)
            .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
            .into_model::<VideoPromptSourceRecord>()
            .all(db),
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(generated_image::Column::ImageType.is_in([
                IMAGE_TYPE_VIDEO_KEY_FRAME,
                IMAGE_TYPE_VIDEO_START_FRAME,
                IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
            ]),)
            .order_by_asc(generated_image::Column::Id)
            .all(db),
    );
    let prompts = prompts_result.map_err(AppError::database)?;
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<HashMap<_, _>>();
    let images = images_result.map_err(AppError::database)?;
    let mut records = Vec::new();
    for image in images {
        if let Some(prompt_id) =
            video_frame_image_prompt_id(&image.image_type, image.metadata_json.as_ref())
        {
            let mut record =
                generated_image_record(image.clone(), "video_frame_image", "image_generated");
            if let Some(prompt) = prompt_by_id.get(&prompt_id) {
                record.source_refs.push(lineage_ref(
                    &format!("chapter_video_prompt:{}", prompt.id),
                    "视频首尾帧提示词",
                    "prompt",
                    "video_frame_prompt",
                    Some(prompt.id),
                ));
                record.prompt_sources = video_prompt_sources_from_record(prompt);
            } else {
                record.warnings.push(format!(
                    "图片 metadata 指向的视频提示词 #{} 不存在。",
                    prompt_id
                ));
            }
            records.push(record);
        } else if a4n_domain::image::is_video_frame_prompt_image_type(&image.image_type) {
            warnings.push(format!(
                "视频帧图片 #{} 缺少有效 generation_context.source/prompt_id。",
                image.id
            ));
            let mut record = generated_image_record(image, "video_frame_image", "unknown");
            record
                .warnings
                .push("未找到视频首尾帧提示词来源。".to_string());
            records.push(record);
        }
    }
    Ok(records)
}
