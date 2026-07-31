use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_core::video_execution::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, propagate_adopted_video_change,
    video_segment_recipes_from_metadata,
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
            .filter(generated_video::Column::Status.eq("completed"))
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
            "completed".to_string()
        } else {
            "pending".to_string()
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
        let source_video_id = prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("continuity_source"))
            .and_then(|source| source.get("video_id"))
            .and_then(Value::as_i64);
        let mut metadata = prompt.metadata_json.clone().unwrap_or(Value::Null);
        let mut changed = clear_segment_recipe_video_references(&mut metadata, &video_ids)?;
        if source_video_id.is_some_and(|source_id| video_ids.contains(&source_id)) {
            let metadata = metadata.as_object_mut().ok_or_else(|| {
                VideoDbError::InvalidArgument(format!(
                    "视频提示词 #{} metadata_json 必须为对象",
                    prompt.id
                ))
            })?;
            metadata.insert("continuity_stale".to_string(), Value::Bool(true));
            metadata.insert(
                "continuity_stale_reason".to_string(),
                Value::String("continuity_source_deleted".to_string()),
            );
            changed = true;
        }
        if !changed {
            continue;
        }
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
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
}
