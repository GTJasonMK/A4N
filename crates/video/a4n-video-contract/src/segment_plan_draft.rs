use std::collections::BTreeSet;

use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2,
    validate_video_segment_recipes_for_plan,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentPlanDraftCheckpoint {
    pub step_key: String,
    pub extra_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub artifacts_json: Option<Value>,
}

pub fn load_video_segment_plan_draft_recipe_state(
    checkpoints: &[VideoSegmentPlanDraftCheckpoint],
    style: &str,
) -> Result<
    (
        Vec<VideoSegmentPlanV2>,
        Vec<VideoSegmentExecutionRecipeV1>,
        usize,
        usize,
    ),
    AppError,
> {
    let mut groups = checkpoints
        .iter()
        .map(|checkpoint| {
            let context = format!("片段规划 checkpoint {}", checkpoint.step_key);
            let shot_index = checkpoint
                .extra_json
                .as_ref()
                .and_then(|value| value.get("shot_index"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_shot_index".to_string(),
                    operation: format!("读取{context}"),
                })?;
            let total_shots = checkpoint
                .extra_json
                .as_ref()
                .and_then(|value| value.get("total_shots"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_total_shots".to_string(),
                    operation: format!("读取{context}"),
                })?;
            let segments =
                checkpoint_segment_plans(checkpoint.parsed_output_json.as_ref(), &context)?;
            let recipes = checkpoint_recipes(
                checkpoint.artifacts_json.as_ref(),
                &segments,
                style,
                &context,
            )?;
            Ok((shot_index, total_shots, segments, recipes))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if groups.is_empty() {
        return Err(AppError::InvalidState {
            current: "missing_segment_plan_shot_checkpoint".to_string(),
            operation: "读取片段规划草稿执行方案".to_string(),
        });
    }
    let total_shot_count = groups[0].1;
    if groups
        .iter()
        .any(|(_, total_shots, ..)| *total_shots != total_shot_count)
    {
        return Err(AppError::InvalidState {
            current: "inconsistent_total_shots".to_string(),
            operation: "读取片段规划草稿执行方案".to_string(),
        });
    }
    if groups.len() > total_shot_count {
        return Err(AppError::InvalidState {
            current: format!(
                "completed_shots_exceed_total:{}>{total_shot_count}",
                groups.len()
            ),
            operation: "读取片段规划草稿执行方案".to_string(),
        });
    }
    let mut seen_shot_indexes = BTreeSet::new();
    for (shot_index, ..) in &groups {
        if *shot_index > total_shot_count {
            return Err(AppError::InvalidState {
                current: format!("shot_index_exceeds_total:{shot_index}>{total_shot_count}"),
                operation: "读取片段规划草稿执行方案".to_string(),
            });
        }
        if !seen_shot_indexes.insert(*shot_index) {
            return Err(AppError::InvalidState {
                current: format!("duplicate_shot_index:{shot_index}"),
                operation: "读取片段规划草稿执行方案".to_string(),
            });
        }
    }
    groups.sort_by_key(|(shot_index, ..)| *shot_index);

    let mut next_segment_number = 1i32;
    let mut segments = Vec::new();
    let mut recipes = Vec::new();
    for (_, _, group_segments, group_recipes) in groups {
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
    Ok((segments, recipes, checkpoints.len(), total_shot_count))
}

pub fn update_video_segment_plan_draft_recipe_artifacts(
    parsed_output_json: Option<&Value>,
    artifacts_json: Option<&Value>,
    source_shot_id: &str,
    segment_index_in_shot: i32,
    style: &str,
    mut recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Value, AppError> {
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
    let context = format!("片段规划 checkpoint {source_shot_id}");
    let segments = checkpoint_segment_plans(parsed_output_json, &context)?;
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
    let mut recipes = checkpoint_recipes(artifacts_json, &segments, style, &context)?;
    recipe.segment_number = segment.segment_number;
    recipe
        .validate_for_plan(segment, style)
        .map_err(AppError::InvalidArgument)?;
    recipes[segment_index] = recipe;
    validate_video_segment_recipes_for_plan(&recipes, &segments, style)
        .map_err(AppError::InvalidArgument)?;
    artifacts_with_recipes(artifacts_json, recipes, &context)
}

fn checkpoint_segment_plans(
    parsed_output_json: Option<&Value>,
    context: &str,
) -> Result<Vec<VideoSegmentPlanV2>, AppError> {
    let values = parsed_output_json
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_validated_segments".to_string(),
            operation: format!("读取{context}"),
        })?;
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
    artifacts_json: Option<&Value>,
    segments: &[VideoSegmentPlanV2],
    style: &str,
    context: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
    let recipes = artifacts_json
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
    let mut artifacts = current
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} artifacts_json 必须是对象")))?;
    artifacts.insert(
        VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
        serde_json::to_value(recipes)?,
    );
    Ok(Value::Object(artifacts))
}
