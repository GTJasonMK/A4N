use super::ImageReferenceCleanupIds;
use crate::video::VideoDbError;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter_manga_prompt, chapter_video_prompt},
};
use a4n_video_contract::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
    propagate_adopted_video_change, video_segment_recipes_from_metadata,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    sea_query::{Alias, CaseStatement, Expr},
};
use serde_json::Value;
use std::collections::HashSet;

fn clear_segment_recipe_image_references(
    metadata: &mut Value,
    image_ids: &HashSet<i64>,
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
    image_ids: &HashSet<i64>,
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
) -> Result<ImageReferenceCleanupIds, VideoDbError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(ImageReferenceCleanupIds {
            updated_prompt_ids: Vec::new(),
            updated_video_ids: Vec::new(),
        });
    }

    let now = Utc::now().naive_utc();
    chapter_manga_prompt::Entity::update_many()
        .col_expr(
            chapter_manga_prompt::Column::GeneratedImageId,
            Expr::value(Option::<i64>::None),
        )
        .col_expr(chapter_manga_prompt::Column::Status, Expr::value("pending"))
        .col_expr(chapter_manga_prompt::Column::UpdatedAt, Expr::value(now))
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_in(image_ids.iter().copied()))
        .exec(db)
        .await?;

    let updated_video_ids = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .filter(generated_video::Column::SourceImageId.is_in(image_ids.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    generated_video::Entity::update_many()
        .col_expr(
            generated_video::Column::SourceImageId,
            Expr::value(Option::<i64>::None),
        )
        .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
        .filter(generated_video::Column::SourceImageId.is_in(image_ids.iter().copied()))
        .exec(db)
        .await?;

    let deleted_image_ids = image_ids.iter().copied().collect::<HashSet<_>>();
    // Derived video projects may reference root-project images, so project
    // scope is not sufficient. Chapter-bound images can still be narrowed by
    // chapter; reusable images and missing IDs keep the legacy full scan.
    let image_contexts = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ChapterId)
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .into_tuple::<(i64, Option<i64>)>()
        .all(db)
        .await?;
    let scoped_chapter_ids = (image_contexts.len() == deleted_image_ids.len()
        && image_contexts
            .iter()
            .all(|(_, chapter_id)| chapter_id.is_some()))
    .then(|| {
        image_contexts
            .into_iter()
            .filter_map(|(_, chapter_id)| chapter_id)
            .collect::<HashSet<_>>()
    });
    let mut plans = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::MetadataJson)
        .filter(
            chapter_video_prompt::Column::PromptType
                .eq(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN),
        )
        .filter(
            chapter_video_prompt::Column::WorkflowMode
                .eq(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN),
        );
    if let Some(chapter_ids) = scoped_chapter_ids.as_ref() {
        plans = plans
            .filter(chapter_video_prompt::Column::ChapterId.is_in(chapter_ids.iter().copied()));
    }
    let plans = plans.into_tuple::<(i64, Option<Value>)>().all(db).await?;
    let mut plan_updates = Vec::new();
    let mut updated_prompt_ids = Vec::new();
    for (plan_id, metadata_json) in plans {
        let Some(mut metadata) = metadata_json else {
            continue;
        };
        if !clear_segment_recipe_image_references(&mut metadata, &deleted_image_ids)? {
            continue;
        }
        plan_updates.push((plan_id, metadata));
        updated_prompt_ids.push(plan_id);
    }
    if !plan_updates.is_empty() {
        let mut metadata_case = CaseStatement::new();
        let mut plan_ids = Vec::with_capacity(plan_updates.len());
        for (plan_id, metadata) in plan_updates {
            metadata_case = metadata_case.case(
                chapter_video_prompt::Column::Id.eq(plan_id),
                Expr::value(Some(metadata)).cast_as(Alias::new("json")),
            );
            plan_ids.push(plan_id);
        }
        let updated = chapter_video_prompt::Entity::update_many()
            .col_expr(
                chapter_video_prompt::Column::MetadataJson,
                metadata_case
                    .finally(Expr::col(chapter_video_prompt::Column::MetadataJson))
                    .into(),
            )
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now))
            .filter(chapter_video_prompt::Column::Id.is_in(plan_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != plan_ids.len() as u64 {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频图片引用批量更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                plan_ids.len()
            )));
        }
    }

    let mut inserted_prompts = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::MetadataJson)
        .filter(chapter_video_prompt::Column::TimelineItemId.is_not_null());
    if let Some(chapter_ids) = scoped_chapter_ids.as_ref() {
        inserted_prompts = inserted_prompts
            .filter(chapter_video_prompt::Column::ChapterId.is_in(chapter_ids.iter().copied()));
    }
    let inserted_prompts = inserted_prompts
        .into_tuple::<(i64, Option<Value>)>()
        .all(db)
        .await?;
    let mut inserted_updates = Vec::new();
    for (prompt_id, metadata_json) in inserted_prompts {
        let Some(mut metadata) = metadata_json else {
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
        inserted_updates.push((
            prompt_id,
            metadata,
            serde_json::json!(recipe.provider_reference_image_ids()),
        ));
        updated_prompt_ids.push(prompt_id);
    }
    if !inserted_updates.is_empty() {
        let mut metadata_case = CaseStatement::new();
        let mut reference_image_ids_case = CaseStatement::new();
        let mut prompt_ids = Vec::with_capacity(inserted_updates.len());
        for (prompt_id, metadata, reference_image_ids_json) in inserted_updates {
            metadata_case = metadata_case.case(
                chapter_video_prompt::Column::Id.eq(prompt_id),
                Expr::value(Some(metadata)).cast_as(Alias::new("json")),
            );
            reference_image_ids_case = reference_image_ids_case.case(
                chapter_video_prompt::Column::Id.eq(prompt_id),
                Expr::value(Some(reference_image_ids_json)).cast_as(Alias::new("json")),
            );
            prompt_ids.push(prompt_id);
        }
        let updated = chapter_video_prompt::Entity::update_many()
            .col_expr(
                chapter_video_prompt::Column::MetadataJson,
                metadata_case
                    .finally(Expr::col(chapter_video_prompt::Column::MetadataJson))
                    .into(),
            )
            .col_expr(
                chapter_video_prompt::Column::ReferenceImageIdsJson,
                reference_image_ids_case
                    .finally(Expr::col(
                        chapter_video_prompt::Column::ReferenceImageIdsJson,
                    ))
                    .into(),
            )
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now))
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != prompt_ids.len() as u64 {
            return Err(VideoDbError::InvalidArgument(format!(
                "插入片段图片引用批量更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                prompt_ids.len()
            )));
        }
    }

    crate::canvas::mark_canvas_generated_image_refs_missing(db, image_ids).await?;
    updated_prompt_ids.sort_unstable();
    updated_prompt_ids.dedup();
    Ok(ImageReferenceCleanupIds {
        updated_prompt_ids,
        updated_video_ids,
    })
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
            clear_segment_recipe_image_references(&mut metadata, &HashSet::from([11]),).unwrap()
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
            clear_inserted_prompt_image_references(&mut metadata, &HashSet::from([11])).unwrap()
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
