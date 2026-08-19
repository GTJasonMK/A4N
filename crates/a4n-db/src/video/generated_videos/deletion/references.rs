use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_core::video_execution::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
    propagate_adopted_video_change, video_segment_recipes_from_metadata,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeMap;

fn clear_source_video_metadata_references(metadata: &mut Value) {
    let Some(obj) = metadata.as_object_mut() else {
        return;
    };
    obj.remove("source_video_id");
    if let Some(trim) = obj.get_mut("trim")
        && let Some(trim_obj) = trim.as_object_mut()
    {
        trim_obj.remove("source_video_id");
    }
}

fn clear_segment_recipe_video_references(
    metadata: &mut Value,
    video_ids: &[i64],
) -> Result<bool, VideoDbError> {
    let mut recipes = video_segment_recipes_from_metadata(Some(metadata)).map_err(|error| {
        VideoDbError::InvalidArgument(format!(
            "清理视频引用时发现片段执行方案 metadata 无效：{error}"
        ))
    })?;
    if recipes.is_empty() {
        return Ok(false);
    }
    let mut changed = false;
    for index in 0..recipes.len() {
        let (segment_number, invalidated_adopted_video_id, recipe_changed) = {
            let recipe = &mut recipes[index];
            let adopted_deleted = recipe
                .adopted_video_id
                .is_some_and(|video_id| video_ids.contains(&video_id));
            let continuity_deleted = recipe
                .continuity_source_video_id
                .is_some_and(|video_id| video_ids.contains(&video_id));
            let invalidated_adopted_video_id = (adopted_deleted || continuity_deleted)
                .then(|| recipe.adopted_video_id.take())
                .flatten();
            if continuity_deleted {
                recipe.continuity_source_video_id = None;
            }
            (
                recipe.segment_number,
                invalidated_adopted_video_id,
                adopted_deleted || continuity_deleted,
            )
        };
        if recipe_changed {
            changed = true;
            propagate_adopted_video_change(
                &mut recipes,
                segment_number,
                invalidated_adopted_video_id,
                None,
            );
        }
    }
    if changed {
        let object = metadata.as_object_mut().ok_or_else(|| {
            VideoDbError::InvalidArgument(
                "片段规划 metadata_json 必须为对象，不能清理视频引用".to_string(),
            )
        })?;
        object.insert(
            VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
            serde_json::to_value(recipes).map_err(|error| {
                VideoDbError::InvalidArgument(format!("序列化已清理的片段执行方案失败：{error}"))
            })?,
        );
    }
    Ok(changed)
}

fn clear_inserted_prompt_video_reference(
    metadata: &mut Value,
    video_ids: &[i64],
) -> Result<bool, VideoDbError> {
    let Some(value) = metadata.get("segment_recipe").cloned() else {
        return Ok(false);
    };
    let mut recipe =
        serde_json::from_value::<VideoSegmentExecutionRecipeV1>(value).map_err(|error| {
            VideoDbError::InvalidArgument(format!(
                "清理插入片段视频引用时发现 segment_recipe 无效：{error}"
            ))
        })?;
    if !recipe
        .continuity_source_video_id
        .is_some_and(|video_id| video_ids.contains(&video_id))
    {
        return Ok(false);
    }
    recipe.continuity_source_video_id = None;
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

pub(crate) async fn clear_generated_video_references_record<C>(
    db: &C,
    videos: &[generated_video::Model],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let video_projects = videos
        .iter()
        .map(|video| (video.id, video.project_id))
        .collect::<BTreeMap<_, _>>();
    if video_projects.is_empty() {
        return Ok(());
    }
    let video_ids = video_projects.keys().copied().collect::<Vec<_>>();
    let now = Utc::now().naive_utc();

    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::GeneratedVideoId.is_in(video_ids.iter().copied()))
        .all(db)
        .await?;
    for prompt in prompts {
        let replacement = generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(prompt.project_id))
            .filter(generated_video::Column::ChapterId.eq(prompt.chapter_id))
            .filter(generated_video::Column::Status.eq(generated_video::STATUS_COMPLETED))
            .all(db)
            .await?
            .into_iter()
            .filter(|video| !video_ids.contains(&video.id))
            .filter(|video| {
                video
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("prompt_id"))
                    .and_then(Value::as_i64)
                    == Some(prompt.id)
            })
            .max_by_key(|video| (video.updated_at, video.id));
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.generated_video_id = Set(replacement.as_ref().map(|video| video.id));
        active.status = Set(if replacement.is_some() {
            chapter_video_prompt::STATUS_COMPLETED.to_string()
        } else {
            chapter_video_prompt::STATUS_PENDING.to_string()
        });
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    let project_ids = video_projects.values().copied().collect::<Vec<_>>();
    let referenced_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids))
        .all(db)
        .await?;
    for prompt in referenced_prompts {
        let mut metadata = prompt.metadata_json.clone().unwrap_or(Value::Null);
        let mut changed = clear_segment_recipe_video_references(&mut metadata, &video_ids)?;
        changed |= clear_inserted_prompt_video_reference(&mut metadata, &video_ids)?;
        if !changed {
            continue;
        }
        let inserted = prompt.timeline_item_id.is_some();
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        if inserted {
            let recipe = serde_json::from_value::<VideoSegmentExecutionRecipeV1>(
                metadata.get("segment_recipe").cloned().ok_or_else(|| {
                    VideoDbError::InvalidArgument("已清理的插入片段缺少 segment_recipe".to_string())
                })?,
            )
            .map_err(|error| {
                VideoDbError::InvalidArgument(format!("读取已清理插入片段执行方案失败：{error}"))
            })?;
            active.reference_image_ids_json = Set(Some(serde_json::json!(
                recipe.provider_reference_image_ids()
            )));
        }
        active.metadata_json = Set(Some(metadata));
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    let child_videos = generated_video::Entity::find()
        .filter(generated_video::Column::SourceVideoId.is_in(video_ids.iter().copied()))
        .all(db)
        .await?;
    for video in child_videos {
        let mut metadata = video.metadata_json.clone();
        let mut active: generated_video::ActiveModel = video.into();
        active.source_video_id = Set(None);
        if let Some(mut metadata) = metadata.take() {
            clear_source_video_metadata_references(&mut metadata);
            active.metadata_json = Set(Some(metadata));
        }
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    for (video_id, project_id) in video_projects {
        crate::canvas::mark_canvas_generated_video_refs_missing(db, project_id, video_id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn source_video_metadata_cleanup_preserves_unrelated_fields() {
        let mut metadata = json!({
            "source_video_id": 41,
            "provider": "test",
            "trim": {
                "source_video_id": 41,
                "start_secs": 1.5
            }
        });

        clear_source_video_metadata_references(&mut metadata);

        assert_eq!(
            metadata,
            json!({
                "provider": "test",
                "trim": {
                    "start_secs": 1.5
                }
            })
        );
    }

    #[test]
    fn segment_recipe_cleanup_invalidates_the_full_continuity_chain() {
        let mut metadata = json!({
            "segment_recipes": [
                {
                    "schema_version": "video_segment_execution_recipe_v1",
                    "segment_number": 1,
                    "workflow_mode": "first_frame_chain",
                    "style": "cinematic",
                    "prompt_output_language": "zh_cn",
                    "duration_secs": 6,
                    "duration_secs_options": [6],
                    "aspect_ratio": "16:9",
                    "reference_image_ids": [],
                    "video_library_image_ids": [],
                    "guidance_mode": "none",
                    "guidance_reference_image_ids": [],
                    "continuity_source_video_id": null,
                    "reference_media": null,
                    "adopted_video_id": 41
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
                    "continuity_source_video_id": 41,
                    "reference_media": null,
                    "adopted_video_id": 42
                },
                {
                    "schema_version": "video_segment_execution_recipe_v1",
                    "segment_number": 3,
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
                    "continuity_source_video_id": 42,
                    "reference_media": null,
                    "adopted_video_id": 43
                }
            ]
        });

        assert!(clear_segment_recipe_video_references(&mut metadata, &[41]).unwrap());
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
        assert_eq!(
            metadata["segment_recipes"][2]["continuity_source_video_id"],
            Value::Null
        );
        assert_eq!(
            metadata["segment_recipes"][2]["adopted_video_id"],
            Value::Null
        );
    }

    #[test]
    fn inserted_prompt_cleanup_only_clears_recipe_continuity_source() {
        let mut metadata = json!({
            "source_prompt_type": "inserted_segment",
            "segment_recipe": {
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
                "continuity_source_video_id": 41,
                "reference_media": null,
                "adopted_video_id": null
            },
            "video_execution_identity": {"segment_sha256": "stable"}
        });

        assert!(clear_inserted_prompt_video_reference(&mut metadata, &[41]).unwrap());
        assert_eq!(
            metadata["segment_recipe"]["continuity_source_video_id"],
            Value::Null
        );
        assert_eq!(
            metadata["video_execution_identity"],
            json!({"segment_sha256": "stable"})
        );
        assert!(metadata.get("continuity_stale").is_none());
    }
}
