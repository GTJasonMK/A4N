use std::collections::{BTreeSet, HashSet};

use a4n_core::video_execution::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2,
    carry_forward_video_segment_recipes, default_video_segment_recipes,
    validate_video_segment_recipes_for_plan, video_segment_recipes_for_plan_from_metadata,
    video_segments_share_recipe_anchor,
};
use a4n_db::{
    video::VideoSegmentPlanDraftCheckpoint,
    workflow::{WORKFLOW_STATUS_COMPLETED, WorkflowWriteError},
};
use serde_json::{Value, json};

use super::super::{
    AppError, OrchestratorContext, PromptWorkflowMode, SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    VideoSegmentPlan, segment_plan_rows,
};

use crate::{video::video_db_error, workflow::workflow_write_app_error};

const SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX: &str = "video_segment_plan.shot.";
const SEGMENT_PLAN_FINAL_CHECKPOINT_KEY: &str = "video_segment_plan.final";

pub(in crate::video::prompt_generation::frame_prompts) async fn load_previous_shared_segment_recipe_state(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    load_existing: bool,
) -> Result<Option<(Vec<VideoSegmentPlan>, Vec<VideoSegmentExecutionRecipeV1>)>, AppError> {
    if !load_existing {
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

pub(in crate::video) async fn load_video_segment_plan_draft_checkpoints(
    service: &OrchestratorContext,
    workflow_run_id: i64,
) -> Result<Vec<VideoSegmentPlanDraftCheckpoint>, AppError> {
    a4n_db::video::video_segment_plan_draft_checkpoints(
        service.database.connection(),
        workflow_run_id,
        SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX,
    )
    .await
    .map_err(video_db_error)
}

pub(in crate::video) async fn update_video_segment_plan_draft_recipe_checkpoint(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    source_shot_id: &str,
    segment_index_in_shot: i32,
    style: &str,
    recipe: VideoSegmentExecutionRecipeV1,
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
    let context = format!("片段规划 checkpoint {step_key}");
    a4n_db::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        service.database.connection(),
        workflow_run_id,
        &step_key,
        SEGMENT_PLAN_FINAL_CHECKPOINT_KEY,
        move |parsed_output_json, artifacts_json| {
            let segments = checkpoint_segment_plans(parsed_output_json, &context)
                .map_err(draft_recipe_mutation_error)?;
            let segment = segments.get(segment_index).ok_or_else(|| {
                WorkflowWriteError::InvalidArgument(format!(
                    "镜头 {source_shot_id} 不存在第 {segment_index_in_shot} 个片段"
                ))
            })?;
            if segment.source_shot_id != source_shot_id {
                return Err(WorkflowWriteError::StatusConflict {
                    current: segment.source_shot_id.clone(),
                    operation: format!("更新片段规划镜头 {source_shot_id} 的执行方案"),
                });
            }
            let mut recipes = checkpoint_recipes(artifacts_json, &segments, style, &context)
                .map_err(draft_recipe_mutation_error)?;
            let mut recipe = recipe;
            recipe.segment_number = segment.segment_number;
            recipe
                .validate_for_plan(segment, style)
                .map_err(WorkflowWriteError::InvalidArgument)?;
            recipes[segment_index] = recipe;
            validate_video_segment_recipes_for_plan(&recipes, &segments, style)
                .map_err(WorkflowWriteError::InvalidArgument)?;
            artifacts_with_recipes(artifacts_json, recipes, &context)
                .map_err(draft_recipe_mutation_error)
        },
    )
    .await
    .map_err(workflow_write_app_error)
}

pub(in crate::video) fn load_video_segment_plan_draft_recipe_state(
    checkpoints: &[VideoSegmentPlanDraftCheckpoint],
    style: &str,
) -> Result<
    (
        Vec<VideoSegmentPlan>,
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
            .any(|(draft, final_segment)| !video_segments_share_recipe_anchor(draft, final_segment))
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

pub(in crate::video::prompt_generation::frame_prompts) fn select_video_segment_recipe_state_by_shots(
    segments: Vec<VideoSegmentPlan>,
    recipes: Vec<VideoSegmentExecutionRecipeV1>,
    included_source_shot_ids: &HashSet<String>,
    style: &str,
) -> Result<(Vec<VideoSegmentPlan>, Vec<VideoSegmentExecutionRecipeV1>), AppError> {
    let mut selected = segments
        .into_iter()
        .filter(|segment| included_source_shot_ids.contains(&segment.source_shot_id))
        .map(|segment| {
            let recipe = recipes
                .iter()
                .find(|recipe| recipe.segment_number == segment.segment_number)
                .cloned()
                .ok_or_else(|| AppError::InvalidState {
                    current: format!("missing_recipe_for_segment:{}", segment.segment_number),
                    operation: "筛选片段规划草稿执行方案".to_string(),
                })?;
            Ok((segment, recipe))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    for (index, (segment, recipe)) in selected.iter_mut().enumerate() {
        let segment_number = i32::try_from(index + 1)
            .map_err(|_| AppError::InvalidArgument("片段数量超出 i32 范围".to_string()))?;
        segment.segment_number = segment_number;
        recipe.segment_number = segment_number;
    }
    let (segments, recipes): (Vec<_>, Vec<_>) = selected.into_iter().unzip();
    validate_video_segment_recipes_for_plan(&recipes, &segments, style)
        .map_err(AppError::InvalidArgument)?;
    Ok((segments, recipes))
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
    parsed_output_json: Option<&Value>,
    context: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let values = parsed_output_json
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
    artifacts_json: Option<&Value>,
    segments: &[VideoSegmentPlan],
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

fn draft_recipe_mutation_error(error: AppError) -> WorkflowWriteError {
    match error {
        AppError::InvalidArgument(message) => WorkflowWriteError::InvalidArgument(message),
        AppError::InvalidState { current, operation } => {
            WorkflowWriteError::StatusConflict { current, operation }
        }
        AppError::NotFound(message) => WorkflowWriteError::NotFound(message),
        AppError::Database(error) => WorkflowWriteError::Database(error),
        other => WorkflowWriteError::InvalidData(other.to_string()),
    }
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

    fn draft_checkpoint(
        shot_index: usize,
        total_shots: usize,
        segments: Vec<VideoSegmentPlan>,
    ) -> VideoSegmentPlanDraftCheckpoint {
        let recipes = default_video_segment_recipes(&segments, "电影感", "zh_cn").unwrap();
        VideoSegmentPlanDraftCheckpoint {
            step_key: format!("video_segment_plan.shot.shot-{shot_index}"),
            extra_json: Some(json!({
                "shot_index": shot_index,
                "total_shots": total_shots,
            })),
            parsed_output_json: Some(serde_json::to_value(segments).unwrap()),
            artifacts_json: Some(json!({
                VIDEO_SEGMENT_RECIPES_METADATA_KEY: recipes,
            })),
        }
    }

    #[test]
    fn draft_state_is_ordered_once_on_the_backend() {
        let mut first = vec![segment("shot-1", 1), segment("shot-1", 2)];
        first[0].segment_index_in_shot = 1;
        first[0].shot_segment_count = 2;
        first[1].segment_index_in_shot = 2;
        first[1].shot_segment_count = 2;
        let checkpoints = vec![
            draft_checkpoint(2, 3, vec![segment("shot-2", 1)]),
            draft_checkpoint(1, 3, first),
        ];

        let (segments, recipes, completed_shot_count, total_shot_count) =
            load_video_segment_plan_draft_recipe_state(&checkpoints, "电影感").unwrap();

        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.source_shot_id.as_str())
                .collect::<Vec<_>>(),
            ["shot-1", "shot-1", "shot-2"]
        );
        assert_eq!(
            recipes
                .iter()
                .map(|recipe| recipe.segment_number)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(completed_shot_count, 2);
        assert_eq!(total_shot_count, 3);
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

    #[test]
    fn selecting_draft_shots_drops_stale_checkpoints_and_renumbers_recipes() {
        let first = segment("shot-1", 1);
        let mut second = segment("shot-2", 2);
        second.segment_index_in_shot = 1;
        let segments = vec![first, second];
        let recipes = default_video_segment_recipes(&segments, "电影感", "zh_cn").unwrap();

        let (selected_segments, selected_recipes) = select_video_segment_recipe_state_by_shots(
            segments,
            recipes,
            &HashSet::from(["shot-2".to_string()]),
            "电影感",
        )
        .unwrap();

        assert_eq!(selected_segments.len(), 1);
        assert_eq!(selected_segments[0].source_shot_id, "shot-2");
        assert_eq!(selected_segments[0].segment_number, 1);
        assert_eq!(selected_recipes[0].segment_number, 1);
    }
}
