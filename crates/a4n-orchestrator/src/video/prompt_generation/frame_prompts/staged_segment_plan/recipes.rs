use std::collections::BTreeSet;

use a4n_core::video_execution::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2,
    carry_forward_video_segment_recipes, default_video_segment_recipes,
    validate_video_segment_recipes_for_plan, video_segment_recipes_for_plan_from_metadata,
};
use a4n_db::workflow::WORKFLOW_STATUS_COMPLETED;
use serde_json::{Map, Value, json};

use super::super::{
    AppError, OrchestratorContext, PromptWorkflowMode, SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    VideoSegmentPlan, segment_plan_rows,
};

const SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX: &str = "video_segment_plan.shot.";

pub(in crate::video) async fn load_previous_shared_segment_recipe_state(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    regenerate_existing: bool,
) -> Result<Option<(Vec<VideoSegmentPlan>, Vec<VideoSegmentExecutionRecipeV1>)>, AppError> {
    if !regenerate_existing {
        return Ok(None);
    }
    let previous = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        PromptWorkflowMode::SharedSegmentPlan,
        SHARED_SEGMENT_PLAN_PROMPT_TYPE,
        false,
    )
    .await?
    .into_iter()
    .filter(|row| row.status == WORKFLOW_STATUS_COMPLETED)
    .max_by_key(|row| row.id);
    previous
        .map(|row| {
            let metadata = row.metadata_json.as_ref().ok_or_else(|| {
                AppError::InvalidArgument("已有共享片段规划缺少 metadata_json".to_string())
            })?;
            let previous_segments = serde_json::from_value::<Vec<VideoSegmentPlan>>(
                metadata.get("segments").cloned().ok_or_else(|| {
                    AppError::InvalidArgument("已有共享片段规划缺少 segments".to_string())
                })?,
            )
            .map_err(|error| {
                AppError::InvalidArgument(format!("已有共享片段规划 segments 无效：{error}"))
            })?;
            let previous_style = row
                .style
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument("已有共享片段规划缺少 style".to_string())
                })?;
            let previous_recipes =
                video_segment_recipes_for_plan_from_metadata(Some(metadata), previous_style)
                    .map_err(AppError::InvalidArgument)?;
            Ok((previous_segments, previous_recipes))
        })
        .transpose()
}

pub(super) fn segment_plan_shot_recipe_artifacts(
    segment_values: &[Value],
    style: &str,
    prompt_output_language: &str,
    previous_recipe_state: Option<(&[VideoSegmentPlan], &[VideoSegmentExecutionRecipeV1])>,
) -> Result<Value, AppError> {
    let segments = segment_plans_from_values(segment_values, "视频片段规划镜头")?;
    let recipes = initial_recipes_for_segments(
        &segments,
        style,
        prompt_output_language,
        previous_recipe_state,
    )?;
    Ok(json!({ VIDEO_SEGMENT_RECIPES_METADATA_KEY: recipes }))
}

pub(in crate::video) async fn materialize_video_segment_plan_draft_recipes(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    style: &str,
    prompt_output_language: &str,
    previous_recipe_state: Option<(&[VideoSegmentPlan], &[VideoSegmentExecutionRecipeV1])>,
) -> Result<(), AppError> {
    for checkpoint in service.workflow_checkpoint_models(workflow_run_id).await? {
        if checkpoint.status != WORKFLOW_STATUS_COMPLETED
            || !is_segment_plan_shot_checkpoint(&checkpoint.step_key)
        {
            continue;
        }
        let context = format!("片段规划 checkpoint {}", checkpoint.step_key);
        let segments = checkpoint_segment_plans(&checkpoint, &context)?;
        if checkpoint
            .artifacts_json
            .as_ref()
            .and_then(Value::as_object)
            .is_some_and(|artifacts| artifacts.contains_key(VIDEO_SEGMENT_RECIPES_METADATA_KEY))
        {
            checkpoint_recipes(&checkpoint, &segments, style, &context)?;
            continue;
        }
        let recipes = initial_recipes_for_segments(
            &segments,
            style,
            prompt_output_language,
            previous_recipe_state,
        )?;
        let artifacts =
            artifacts_with_recipes(checkpoint.artifacts_json.as_ref(), recipes, &context)?;
        service
            .update_workflow_checkpoint_artifacts(workflow_run_id, &checkpoint.step_key, artifacts)
            .await?;
    }
    Ok(())
}

pub(in crate::video) async fn update_video_segment_plan_draft_recipe_checkpoint(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    source_shot_id: &str,
    segment_index_in_shot: i32,
    style: &str,
    mut recipe: VideoSegmentExecutionRecipeV1,
) -> Result<(), AppError> {
    let source_shot_id = source_shot_id.trim();
    if source_shot_id.is_empty() || source_shot_id.contains(':') {
        return Err(AppError::InvalidArgument(
            "片段规划草稿 source_shot_id 无效".to_string(),
        ));
    }
    let segment_index = usize::try_from(segment_index_in_shot)
        .ok()
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            AppError::InvalidArgument("segment_index_in_shot 必须为正整数".to_string())
        })?;
    let step_key = format!("{SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX}{source_shot_id}");
    let checkpoint = service
        .workflow_checkpoint_model_by_step(workflow_run_id, &step_key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("片段规划草稿不存在镜头 {source_shot_id}")))?;
    if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: checkpoint.status,
            operation: format!("更新片段规划镜头 {source_shot_id} 的执行方案"),
        });
    }
    let context = format!("片段规划 checkpoint {step_key}");
    let segments = checkpoint_segment_plans(&checkpoint, &context)?;
    let segment = segments.get(segment_index).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "镜头 {source_shot_id} 不存在第 {segment_index_in_shot} 个片段"
        ))
    })?;
    if segment.source_shot_id != source_shot_id {
        return Err(AppError::InvalidState {
            current: segment.source_shot_id.clone(),
            operation: format!("更新片段规划镜头 {source_shot_id} 的执行方案"),
        });
    }
    let mut recipes = checkpoint_recipes(&checkpoint, &segments, style, &context)?;
    recipe.segment_number = segment.segment_number;
    recipe
        .validate_for_plan(segment, style)
        .map_err(AppError::InvalidArgument)?;
    recipes[segment_index] = recipe;
    validate_video_segment_recipes_for_plan(&recipes, &segments, style)
        .map_err(AppError::InvalidArgument)?;
    let artifacts = artifacts_with_recipes(checkpoint.artifacts_json.as_ref(), recipes, &context)?;
    service
        .update_workflow_checkpoint_artifacts(workflow_run_id, &step_key, artifacts)
        .await?;
    Ok(())
}

pub(in crate::video) async fn load_video_segment_plan_draft_recipe_state(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    style: &str,
) -> Result<(Vec<VideoSegmentPlan>, Vec<VideoSegmentExecutionRecipeV1>), AppError> {
    let mut groups = service
        .workflow_checkpoint_models(workflow_run_id)
        .await?
        .into_iter()
        .filter(|checkpoint| {
            checkpoint.status == WORKFLOW_STATUS_COMPLETED
                && is_segment_plan_shot_checkpoint(&checkpoint.step_key)
        })
        .map(|checkpoint| {
            let context = format!("片段规划 checkpoint {}", checkpoint.step_key);
            let shot_index = checkpoint
                .input_fingerprint_json
                .as_ref()
                .and_then(|value| value.get("extra"))
                .and_then(|value| value.get("shot_index"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_shot_index".to_string(),
                    operation: format!("读取{context}"),
                })?;
            let segments = checkpoint_segment_plans(&checkpoint, &context)?;
            let recipes = checkpoint_recipes(&checkpoint, &segments, style, &context)?;
            Ok((
                shot_index,
                checkpoint.step_order,
                checkpoint.id,
                segments,
                recipes,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if groups.is_empty() {
        return Err(AppError::InvalidState {
            current: "missing_segment_plan_shot_checkpoint".to_string(),
            operation: "读取片段规划草稿执行方案".to_string(),
        });
    }
    let mut seen_shot_indexes = BTreeSet::new();
    for (shot_index, ..) in &groups {
        if !seen_shot_indexes.insert(*shot_index) {
            return Err(AppError::InvalidState {
                current: format!("duplicate_shot_index:{shot_index}"),
                operation: "读取片段规划草稿执行方案".to_string(),
            });
        }
    }
    groups.sort_by_key(|(shot_index, step_order, id, ..)| (*shot_index, *step_order, *id));

    let mut next_segment_number = 1i32;
    let mut segments = Vec::new();
    let mut recipes = Vec::new();
    for (_, _, _, group_segments, group_recipes) in groups {
        for (mut segment, mut recipe) in group_segments.into_iter().zip(group_recipes) {
            segment.segment_number = next_segment_number;
            recipe.segment_number = next_segment_number;
            next_segment_number = next_segment_number.checked_add(1).ok_or_else(|| {
                AppError::InvalidArgument("片段规划 segment_number 超出 i32 范围".to_string())
            })?;
            segments.push(segment);
            recipes.push(recipe);
        }
    }
    validate_video_segment_recipes_for_plan(&recipes, &segments, style)
        .map_err(AppError::InvalidArgument)?;
    Ok((segments, recipes))
}

pub(in crate::video) fn video_segment_recipes_from_draft_for_final_plan(
    draft_segments: &[VideoSegmentPlan],
    draft_recipes: &[VideoSegmentExecutionRecipeV1],
    final_segments: &[VideoSegmentPlan],
    style: &str,
    prompt_output_language: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
    if draft_segments.len() != final_segments.len()
        || draft_segments
            .iter()
            .zip(final_segments)
            .any(|(draft, final_segment)| !same_segment_anchor(draft, final_segment))
    {
        return Err(AppError::InvalidState {
            current: "segment_plan_draft_anchor_mismatch".to_string(),
            operation: "将草稿执行方案提交到正式片段规划".to_string(),
        });
    }
    carry_forward_video_segment_recipes(
        draft_segments,
        draft_recipes,
        final_segments,
        style,
        prompt_output_language,
    )
    .map_err(AppError::InvalidArgument)
}

fn initial_recipes_for_segments(
    segments: &[VideoSegmentPlan],
    style: &str,
    prompt_output_language: &str,
    previous_recipe_state: Option<(&[VideoSegmentPlan], &[VideoSegmentExecutionRecipeV1])>,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
    match previous_recipe_state {
        Some((previous_segments, previous_recipes)) => carry_forward_video_segment_recipes(
            previous_segments,
            previous_recipes,
            segments,
            style,
            prompt_output_language,
        ),
        None => default_video_segment_recipes(segments, style, prompt_output_language),
    }
    .map_err(AppError::InvalidArgument)
}

fn checkpoint_segment_plans(
    checkpoint: &a4n_core::models::generation::workflow_checkpoint::Model,
    context: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let values = checkpoint
        .checkpoint_json
        .as_ref()
        .and_then(|value| value.get("parsed_output").or_else(|| value.get("output")))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_validated_segments".to_string(),
            operation: format!("读取{context}"),
        })?;
    segment_plans_from_values(values, context)
}

fn segment_plans_from_values(
    values: &[Value],
    context: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(Value::Array(values.to_vec()))
        .map_err(|error| {
            AppError::InvalidArgument(format!("{context} 已校验 segments 无效：{error}"))
        })?;
    if segments.is_empty() {
        return Err(AppError::InvalidState {
            current: "empty_validated_segments".to_string(),
            operation: format!("读取{context}"),
        });
    }
    Ok(segments)
}

fn checkpoint_recipes(
    checkpoint: &a4n_core::models::generation::workflow_checkpoint::Model,
    segments: &[VideoSegmentPlan],
    style: &str,
    context: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
    let recipes = checkpoint
        .artifacts_json
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|artifacts| artifacts.get(VIDEO_SEGMENT_RECIPES_METADATA_KEY))
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_segment_recipes".to_string(),
            operation: format!("读取{context}"),
        })?;
    let recipes =
        serde_json::from_value::<Vec<VideoSegmentExecutionRecipeV1>>(recipes).map_err(|error| {
            AppError::InvalidArgument(format!("{context} segment_recipes 无效：{error}"))
        })?;
    validate_video_segment_recipes_for_plan(&recipes, segments, style)
        .map_err(AppError::InvalidArgument)?;
    Ok(recipes)
}

fn artifacts_with_recipes(
    current: Option<&Value>,
    recipes: Vec<VideoSegmentExecutionRecipeV1>,
    context: &str,
) -> Result<Value, AppError> {
    let mut artifacts = match current {
        Some(Value::Object(object)) => object.clone(),
        Some(_) => {
            return Err(AppError::InvalidArgument(format!(
                "{context} artifacts_json 必须是对象"
            )));
        }
        None => Map::new(),
    };
    artifacts.insert(
        VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
        serde_json::to_value(recipes)?,
    );
    Ok(Value::Object(artifacts))
}

fn is_segment_plan_shot_checkpoint(step_key: &str) -> bool {
    let Some(source_shot_id) = step_key.strip_prefix(SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX) else {
        return false;
    };
    !source_shot_id.is_empty() && !source_shot_id.contains(':')
}

fn same_segment_anchor(left: &VideoSegmentPlan, right: &VideoSegmentPlan) -> bool {
    left.source_shot_id == right.source_shot_id
        && left.segment_index_in_shot == right.segment_index_in_shot
        && left.shot_segment_count == right.shot_segment_count
        && left.covered_beat_ids == right.covered_beat_ids
        && left.source_span_ids == right.source_span_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::video_execution::{VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1};

    fn segment(source_shot_id: &str, segment_number: i32) -> VideoSegmentPlan {
        serde_json::from_value(json!({
            "schema_version": "video_segment_plan_v2",
            "segment_number": segment_number,
            "source_shot_id": source_shot_id,
            "segment_index_in_shot": segment_number,
            "shot_segment_count": 1,
            "covered_beat_ids": [format!("{source_shot_id}-beat")],
            "title": "片段",
            "scene_summary": "场景",
            "blocking_summary": "调度",
            "spatial_anchors": [],
            "emotion_action_goals": [],
            "video_intent": {
                "hook": "钩子",
                "audience_effect": "效果",
                "rhythm": "节奏",
                "visual_priority": "视觉重点"
            },
            "emotional_arc": {
                "start": "开始",
                "turn": "转折",
                "end": "结束",
                "function": "功能"
            },
            "performance_plan": {
                "movement_path": "移动路径",
                "body_performance": "身体表演",
                "gaze_and_hands": "视线和手部",
                "pause_or_reaction": "停顿",
                "camera_relationship": "摄影机关系",
                "time_blocks": [],
                "action_script": []
            },
            "material_coverage": {
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "无"
            },
            "transition_to_next": null,
            "source_span_ids": [format!("{source_shot_id}-span")],
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        }))
        .unwrap()
    }

    #[test]
    fn final_plan_keeps_draft_recipe_by_stable_segment_anchor() {
        let draft_segment = segment("shot-2", 1);
        let mut draft_recipe =
            default_video_segment_recipes(std::slice::from_ref(&draft_segment), "电影感", "zh_cn")
                .unwrap()
                .remove(0);
        draft_recipe.workflow_mode = VideoSegmentWorkflowModeV1::Storyboard;
        draft_recipe.guidance_mode = VideoSegmentGuidanceModeV1::SystemFrames;
        let mut final_segment = draft_segment.clone();
        final_segment.segment_number = 3;

        let recipes = video_segment_recipes_from_draft_for_final_plan(
            &[draft_segment],
            &[draft_recipe],
            &[final_segment],
            "电影感",
            "zh_cn",
        )
        .unwrap();

        assert_eq!(recipes[0].segment_number, 3);
        assert_eq!(
            recipes[0].workflow_mode,
            VideoSegmentWorkflowModeV1::Storyboard
        );
    }
}
