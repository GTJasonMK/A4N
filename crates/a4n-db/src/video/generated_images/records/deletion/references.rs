use crate::video::VideoDbError;
use a4n_core::{
    models::{
        ai::generated_video,
        chapter::{chapter_manga_prompt, chapter_video_prompt},
    },
    video_execution::{
        VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
        propagate_adopted_video_change, video_segment_recipes_from_metadata,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeSet;

fn clear_segment_recipe_image_references(
    metadata: &mut Value,
    image_ids: &BTreeSet<i64>,
) -> Result<bool, VideoDbError> {
    let mut recipes = video_segment_recipes_from_metadata(Some(metadata)).map_err(|error| {
        VideoDbError::InvalidArgument(format!(
            "清理图片引用时发现片段执行方案 metadata 无效：{error}"
        ))
    })?;
    if recipes.is_empty() {
        return Ok(false);
    }
    let mut changed = false;
    let mut invalidated_adoptions = Vec::new();
    for recipe in &mut recipes {
        let before = (
            recipe.reference_image_ids.len(),
            recipe.video_library_image_ids.len(),
            recipe.guidance_reference_image_ids.len(),
        );
        recipe
            .reference_image_ids
            .retain(|id| !image_ids.contains(id));
        recipe
            .video_library_image_ids
            .retain(|id| !image_ids.contains(id));
        recipe
            .guidance_reference_image_ids
            .retain(|id| !image_ids.contains(id));
        let recipe_changed = before
            != (
                recipe.reference_image_ids.len(),
                recipe.video_library_image_ids.len(),
                recipe.guidance_reference_image_ids.len(),
            );
        if recipe_changed {
            changed = true;
            if let Some(video_id) = recipe.adopted_video_id.take() {
                invalidated_adoptions.push((recipe.segment_number, video_id));
            }
        }
    }
    for (segment_number, video_id) in invalidated_adoptions {
        propagate_adopted_video_change(&mut recipes, segment_number, Some(video_id), None);
    }
    if changed {
        metadata
            .as_object_mut()
            .ok_or_else(|| {
                VideoDbError::InvalidArgument(
                    "片段规划 metadata_json 必须为对象，不能清理图片引用".to_string(),
                )
            })?
            .insert(
                VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
                serde_json::to_value(recipes).map_err(|error| {
                    VideoDbError::InvalidArgument(format!(
                        "序列化已清理的片段执行方案失败：{error}"
                    ))
                })?,
            );
    }
    Ok(changed)
}

fn clear_inserted_prompt_image_references(
    metadata: &mut Value,
    image_ids: &BTreeSet<i64>,
) -> Result<bool, VideoDbError> {
    let Some(value) = metadata.get("segment_recipe").cloned() else {
        return Ok(false);
    };
    let mut recipe =
        serde_json::from_value::<VideoSegmentExecutionRecipeV1>(value).map_err(|error| {
            VideoDbError::InvalidArgument(format!(
                "清理插入片段图片引用时发现 segment_recipe 无效：{error}"
            ))
        })?;
    let before = (
        recipe.reference_image_ids.len(),
        recipe.video_library_image_ids.len(),
        recipe.guidance_reference_image_ids.len(),
    );
    recipe
        .reference_image_ids
        .retain(|id| !image_ids.contains(id));
    recipe
        .video_library_image_ids
        .retain(|id| !image_ids.contains(id));
    recipe
        .guidance_reference_image_ids
        .retain(|id| !image_ids.contains(id));
    if before
        == (
            recipe.reference_image_ids.len(),
            recipe.video_library_image_ids.len(),
            recipe.guidance_reference_image_ids.len(),
        )
    {
        return Ok(false);
    }
    metadata
        .as_object_mut()
        .ok_or_else(|| {
            VideoDbError::InvalidArgument("插入片段 metadata_json 必须为对象".to_string())
        })?
        .insert(
            "segment_recipe".to_string(),
            serde_json::to_value(recipe).map_err(|error| {
                VideoDbError::InvalidArgument(format!("序列化插入片段执行方案失败：{error}"))
            })?,
        );
    Ok(true)
}

pub(crate) async fn clear_generated_image_references_record<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(());
    }

    let panels = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_in(image_ids.iter().copied()))
        .all(db)
        .await?;
    for panel in panels {
        let mut active: chapter_manga_prompt::ActiveModel = panel.into();
        active.generated_image_id = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::SourceImageId.is_in(image_ids.iter().copied()))
        .all(db)
        .await?;
    for video in videos {
        let mut active: generated_video::ActiveModel = video.into();
        active.source_image_id = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    let deleted_image_ids = image_ids.iter().copied().collect::<BTreeSet<_>>();
    let plans = chapter_video_prompt::Entity::find()
        .filter(
            chapter_video_prompt::Column::PromptType
                .eq(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN),
        )
        .filter(
            chapter_video_prompt::Column::WorkflowMode
                .eq(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN),
        )
        .all(db)
        .await?;
    for plan in plans {
        let Some(mut metadata) = plan.metadata_json.clone() else {
            continue;
        };
        if !clear_segment_recipe_image_references(&mut metadata, &deleted_image_ids)? {
            continue;
        }
        let mut active: chapter_video_prompt::ActiveModel = plan.into();
        active.metadata_json = Set(Some(metadata));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    let inserted_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::TimelineItemId.is_not_null())
        .all(db)
        .await?;
    for prompt in inserted_prompts {
        let Some(mut metadata) = prompt.metadata_json.clone() else {
            continue;
        };
        if !clear_inserted_prompt_image_references(&mut metadata, &deleted_image_ids)? {
            continue;
        }
        let recipe = serde_json::from_value::<VideoSegmentExecutionRecipeV1>(
            metadata.get("segment_recipe").cloned().ok_or_else(|| {
                VideoDbError::InvalidArgument("已清理的插入片段缺少 segment_recipe".to_string())
            })?,
        )
        .map_err(|error| {
            VideoDbError::InvalidArgument(format!("读取已清理插入片段执行方案失败：{error}"))
        })?;
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.reference_image_ids_json = Set(Some(serde_json::json!(
            recipe.provider_reference_image_ids()
        )));
        active.metadata_json = Set(Some(metadata));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    crate::canvas::mark_canvas_generated_image_refs_missing(db, image_ids).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn segment_recipe_cleanup_removes_references_and_invalidates_adopted_chain() {
        let mut metadata = json!({
            "segment_recipes": [
                {
                    "schema_version": "video_segment_execution_recipe_v1",
                    "segment_number": 1,
                    "workflow_mode": "storyboard",
                    "style": "cinematic",
                    "prompt_output_language": "zh_cn",
                    "duration_secs": 6,
                    "duration_secs_options": [6],
                    "aspect_ratio": "16:9",
                    "reference_image_ids": [11, 12],
                    "video_library_image_ids": [11, 13],
                    "guidance_mode": "keyframes",
                    "guidance_reference_image_ids": [11, 14],
                    "continuity_source_video_id": null,
                    "reference_media": null,
                    "adopted_video_id": 21
                },
                {
                    "schema_version": "video_segment_execution_recipe_v1",
                    "segment_number": 2,
                    "workflow_mode": "first_frame_chain",
                    "style": "cinematic",
                    "prompt_output_language": "zh_cn",
                    "duration_secs": 6,
                    "duration_secs_options": [6],
                    "aspect_ratio": "16:9",
                    "reference_image_ids": [],
                    "video_library_image_ids": [],
                    "guidance_mode": "continuity",
                    "guidance_reference_image_ids": [],
                    "continuity_source_video_id": 21,
                    "reference_media": null,
                    "adopted_video_id": 22
                }
            ]
        });

        assert!(
            clear_segment_recipe_image_references(&mut metadata, &BTreeSet::from([11]),).unwrap()
        );
        assert_eq!(
            metadata["segment_recipes"][0]["reference_image_ids"],
            json!([12])
        );
        assert_eq!(
            metadata["segment_recipes"][0]["video_library_image_ids"],
            json!([13])
        );
        assert_eq!(
            metadata["segment_recipes"][0]["guidance_reference_image_ids"],
            json!([14])
        );
        assert_eq!(
            metadata["segment_recipes"][0]["adopted_video_id"],
            Value::Null
        );
        assert_eq!(
            metadata["segment_recipes"][1]["continuity_source_video_id"],
            Value::Null
        );
        assert_eq!(
            metadata["segment_recipes"][1]["adopted_video_id"],
            Value::Null
        );
    }

    #[test]
    fn inserted_prompt_cleanup_only_updates_segment_recipe() {
        let mut metadata = json!({
            "source_prompt_type": "inserted_segment",
            "segment_recipe": {
                "schema_version": "video_segment_execution_recipe_v1",
                "segment_number": 1,
                "workflow_mode": "boundary_frames",
                "style": "cinematic",
                "prompt_output_language": "zh_cn",
                "duration_secs": 6,
                "duration_secs_options": [6],
                "aspect_ratio": "16:9",
                "reference_image_ids": [11, 12],
                "video_library_image_ids": [13],
                "guidance_mode": "keyframes",
                "guidance_reference_image_ids": [11],
                "continuity_source_video_id": null,
                "reference_media": null,
                "adopted_video_id": null
            },
            "video_execution_identity": {"segment_sha256": "stable"}
        });

        assert!(
            clear_inserted_prompt_image_references(&mut metadata, &BTreeSet::from([11])).unwrap()
        );
        assert_eq!(
            metadata["segment_recipe"]["reference_image_ids"],
            json!([12])
        );
        assert_eq!(
            metadata["segment_recipe"]["guidance_reference_image_ids"],
            json!([])
        );
        assert_eq!(
            metadata["video_execution_identity"],
            json!({"segment_sha256": "stable"})
        );
        assert!(metadata.get("continuity_stale").is_none());
    }
}
