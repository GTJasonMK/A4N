use crate::project_dependency_graph::{
    AssetLineageRecord, generated_image_record, lineage_ref, video_frame_image_prompt_id,
    video_prompt_sources,
};
use a4n_core::{
    error::AppError,
    models::{ai::generated_image, chapter::chapter_video_prompt},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeMap;

pub(in crate::project_dependency_graph) async fn video_frame_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<BTreeMap<_, _>>();
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut records = Vec::new();
    for image in images {
        if let Some(prompt_id) = video_frame_image_prompt_id(&image) {
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
                record.prompt_sources = video_prompt_sources(prompt);
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
