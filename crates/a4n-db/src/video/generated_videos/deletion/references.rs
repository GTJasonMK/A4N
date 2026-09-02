use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_core::video_execution::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
    propagate_adopted_video_change, video_segment_recipes_from_metadata,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    sea_query::{Alias, CaseStatement, Expr},
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
    video_ids: &HashSet<i64>,
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
    video_ids: &HashSet<i64>,
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

pub(crate) async fn clear_generated_video_references_by_ids<C>(
    db: &C,
    video_projects: &[(i64, i64)],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    clear_generated_video_references_for_projects(
        db,
        video_projects.iter().copied().collect::<HashMap<_, _>>(),
    )
    .await
}

async fn clear_generated_video_references_for_projects<C>(
    db: &C,
    video_projects: HashMap<i64, i64>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if video_projects.is_empty() {
        return Ok(());
    }
    let video_ids = video_projects.keys().copied().collect::<Vec<_>>();
    let video_id_set = video_ids.iter().copied().collect::<HashSet<_>>();
    let now = Utc::now().naive_utc();

    let prompts = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .column(chapter_video_prompt::Column::ChapterId)
        .filter(chapter_video_prompt::Column::GeneratedVideoId.is_in(video_ids.iter().copied()))
        .into_tuple::<(i64, i64, i64)>()
        .all(db)
        .await?;
    let mut replacement_by_key = HashMap::<(i64, i64, i64), (i64, chrono::NaiveDateTime)>::new();
    if !prompts.is_empty() {
        let mut chapter_scope = Condition::any();
        let mut project_chapters = HashSet::new();
        for (_, project_id, chapter_id) in &prompts {
            project_chapters.insert((*project_id, *chapter_id));
        }
        for (project_id, chapter_id) in project_chapters {
            chapter_scope = chapter_scope.add(
                Condition::all()
                    .add(generated_video::Column::ProjectId.eq(project_id))
                    .add(generated_video::Column::ChapterId.eq(chapter_id)),
            );
        }
        for (video_id, project_id, chapter_id, metadata_json, updated_at) in
            generated_video::Entity::find()
                .select_only()
                .column(generated_video::Column::Id)
                .column(generated_video::Column::ProjectId)
                .column(generated_video::Column::ChapterId)
                .column(generated_video::Column::MetadataJson)
                .column(generated_video::Column::UpdatedAt)
                .filter(chapter_scope)
                .filter(generated_video::Column::Status.eq(generated_video::STATUS_COMPLETED))
                .filter(generated_video::Column::Id.is_not_in(video_ids.iter().copied()))
                .into_tuple::<(i64, i64, Option<i64>, Option<Value>, chrono::NaiveDateTime)>()
                .all(db)
                .await?
        {
            let Some(chapter_id) = chapter_id else {
                continue;
            };
            let Some(prompt_id) = metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_id"))
                .and_then(Value::as_i64)
            else {
                continue;
            };
            let key = (project_id, chapter_id, prompt_id);
            if replacement_by_key
                .get(&key)
                .is_none_or(|current| (updated_at, video_id) > (current.1, current.0))
            {
                replacement_by_key.insert(key, (video_id, updated_at));
            }
        }
    }
    if !prompts.is_empty() {
        let mut generated_video_id_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut prompt_ids = Vec::with_capacity(prompts.len());
        for (prompt_id, project_id, chapter_id) in &prompts {
            let replacement = replacement_by_key.get(&(*project_id, *chapter_id, *prompt_id));
            generated_video_id_case = generated_video_id_case.case(
                chapter_video_prompt::Column::Id.eq(*prompt_id),
                Expr::value(replacement.map(|video| video.0)),
            );
            status_case = status_case.case(
                chapter_video_prompt::Column::Id.eq(*prompt_id),
                Expr::value(if replacement.is_some() {
                    chapter_video_prompt::STATUS_COMPLETED
                } else {
                    chapter_video_prompt::STATUS_PENDING
                }),
            );
            prompt_ids.push(*prompt_id);
        }
        let updated = chapter_video_prompt::Entity::update_many()
            .col_expr(
                chapter_video_prompt::Column::GeneratedVideoId,
                generated_video_id_case
                    .finally(Expr::col(chapter_video_prompt::Column::GeneratedVideoId))
                    .into(),
            )
            .col_expr(
                chapter_video_prompt::Column::Status,
                status_case
                    .finally(Expr::col(chapter_video_prompt::Column::Status))
                    .into(),
            )
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now))
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != prompt_ids.len() as u64 {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频提示词引用批量更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                prompt_ids.len()
            )));
        }
    }

    let project_ids = video_projects
        .values()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let reference_scope = Condition::any()
        .add(
            Condition::all()
                .add(
                    chapter_video_prompt::Column::PromptType
                        .eq(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN),
                )
                .add(
                    chapter_video_prompt::Column::WorkflowMode
                        .eq(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN),
                ),
        )
        .add(chapter_video_prompt::Column::TimelineItemId.is_not_null());
    let (referenced_prompts_result, child_videos_result) = futures::join!(
        chapter_video_prompt::Entity::find()
            .select_only()
            .column(chapter_video_prompt::Column::Id)
            .column(chapter_video_prompt::Column::TimelineItemId)
            .column(chapter_video_prompt::Column::MetadataJson)
            .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(reference_scope)
            .into_tuple::<(i64, Option<i64>, Option<Value>)>()
            .all(db),
        generated_video::Entity::find()
            .select_only()
            .column(generated_video::Column::Id)
            .column(generated_video::Column::MetadataJson)
            .filter(generated_video::Column::SourceVideoId.is_in(video_id_set.iter().copied()))
            .into_tuple::<(i64, Option<Value>)>()
            .all(db),
    );
    let referenced_prompts = referenced_prompts_result?;
    let mut prompt_updates = Vec::new();
    for (prompt_id, timeline_item_id, metadata_json) in referenced_prompts {
        let mut metadata = metadata_json.unwrap_or(Value::Null);
        let mut changed = clear_segment_recipe_video_references(&mut metadata, &video_id_set)?;
        changed |= clear_inserted_prompt_video_reference(&mut metadata, &video_id_set)?;
        if !changed {
            continue;
        }
        let reference_image_ids_json = if timeline_item_id.is_some() {
            let recipe = serde_json::from_value::<VideoSegmentExecutionRecipeV1>(
                metadata.get("segment_recipe").cloned().ok_or_else(|| {
                    VideoDbError::InvalidArgument("已清理的插入片段缺少 segment_recipe".to_string())
                })?,
            )
            .map_err(|error| {
                VideoDbError::InvalidArgument(format!("读取已清理插入片段执行方案失败：{error}"))
            })?;
            Some(serde_json::json!(recipe.provider_reference_image_ids()))
        } else {
            None
        };
        prompt_updates.push((prompt_id, metadata, reference_image_ids_json));
    }
    if !prompt_updates.is_empty() {
        let mut metadata_case = CaseStatement::new();
        let mut reference_image_ids_case = CaseStatement::new();
        let mut has_reference_image_ids_updates = false;
        let mut prompt_ids = Vec::with_capacity(prompt_updates.len());
        for (prompt_id, metadata, reference_image_ids_json) in prompt_updates {
            metadata_case = metadata_case.case(
                chapter_video_prompt::Column::Id.eq(prompt_id),
                Expr::value(Some(metadata)).cast_as(Alias::new("json")),
            );
            if let Some(reference_image_ids_json) = reference_image_ids_json {
                has_reference_image_ids_updates = true;
                reference_image_ids_case = reference_image_ids_case.case(
                    chapter_video_prompt::Column::Id.eq(prompt_id),
                    Expr::value(Some(reference_image_ids_json)).cast_as(Alias::new("json")),
                );
            }
            prompt_ids.push(prompt_id);
        }
        let mut update = chapter_video_prompt::Entity::update_many()
            .col_expr(
                chapter_video_prompt::Column::MetadataJson,
                metadata_case
                    .finally(Expr::col(chapter_video_prompt::Column::MetadataJson))
                    .into(),
            )
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now));
        if has_reference_image_ids_updates {
            update = update.col_expr(
                chapter_video_prompt::Column::ReferenceImageIdsJson,
                reference_image_ids_case
                    .finally(Expr::col(
                        chapter_video_prompt::Column::ReferenceImageIdsJson,
                    ))
                    .into(),
            );
        }
        let updated = update
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != prompt_ids.len() as u64 {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频提示词片段引用批量更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                prompt_ids.len()
            )));
        }
    }

    let child_videos = child_videos_result?;
    let mut child_updates = Vec::with_capacity(child_videos.len());
    for (video_id, mut metadata) in child_videos {
        if let Some(mut metadata) = metadata.take() {
            clear_source_video_metadata_references(&mut metadata);
            child_updates.push((video_id, Some(metadata)));
        } else {
            child_updates.push((video_id, None));
        }
    }
    if !child_updates.is_empty() {
        let mut metadata_case = CaseStatement::new();
        let mut child_ids = Vec::with_capacity(child_updates.len());
        for (video_id, metadata) in child_updates {
            metadata_case = metadata_case.case(
                generated_video::Column::Id.eq(video_id),
                Expr::value(metadata).cast_as(Alias::new("json")),
            );
            child_ids.push(video_id);
        }
        let updated = generated_video::Entity::update_many()
            .col_expr(
                generated_video::Column::SourceVideoId,
                Expr::value(Option::<i64>::None),
            )
            .col_expr(
                generated_video::Column::MetadataJson,
                metadata_case
                    .finally(Expr::col(generated_video::Column::MetadataJson))
                    .into(),
            )
            .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
            .filter(generated_video::Column::Id.is_in(child_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != child_ids.len() as u64 {
            return Err(VideoDbError::InvalidArgument(format!(
                "子视频来源引用批量更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                child_ids.len()
            )));
        }
    }

    crate::canvas::mark_canvas_generated_video_refs_missing_for_projects_and_videos(
        db,
        &project_ids,
        &video_ids,
    )
    .await?;
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

        assert!(
            clear_segment_recipe_video_references(&mut metadata, &HashSet::from([41])).unwrap()
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

        assert!(
            clear_inserted_prompt_video_reference(&mut metadata, &HashSet::from([41])).unwrap()
        );
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
