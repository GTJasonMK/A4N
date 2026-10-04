use std::collections::{HashMap, HashSet};

use a4n_db::video::VideoSegmentPlanDraftCheckpoint;
use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_video_contract::{
    VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2,
    carry_forward_video_segment_recipes, default_video_segment_recipes,
    validate_video_segment_recipes_for_plan, video_segment_recipes_from_metadata,
    video_segments_share_recipe_anchor,
};
use serde_json::{Value, json};

use super::super::{
    AppError, OrchestratorContext, PromptWorkflowMode, VideoSegmentPlan, segment_plan_rows,
};

use crate::video::video_db_error;

const SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX: &str = "video_segment_plan.shot.";

pub(in crate::video::prompts::generation::frame_prompts) async fn load_previous_shared_segment_recipe_state(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    load_existing: bool,
) -> Result<
    Option<(
        Vec<VideoSegmentPlan>,
        Vec<VideoSegmentExecutionRecipeV1>,
        a4n_video_contract::VideoShootingScriptIdentityV1,
    )>,
    AppError,
> {
    if !load_existing {
        return Ok(None);
    }
    let previous = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        PromptWorkflowMode::SharedSegmentPlan,
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        false,
    )
    .await?
    .into_iter()
    .filter(|row| row.status == chapter_video_prompt::STATUS_COMPLETED)
    .max_by_key(|row| row.id);
    previous
        .map(|row| {
            let metadata = row.metadata_json.as_ref().ok_or_else(|| {
                AppError::InvalidArgument("已有共享片段规划缺少 metadata_json".to_string())
            })?;
            let previous_segments = a4n_video_contract::video_segment_plan_from_metadata(metadata)
                .map_err(AppError::InvalidArgument)?;
            a4n_video_contract::validate_video_segment_plan_identity(
                &metadata["video_execution_identity"],
                &previous_segments,
            )
            .map_err(AppError::InvalidArgument)?;
            let previous_style = row
                .style
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument("已有共享片段规划缺少 style".to_string())
                })?;
            let previous_recipes = video_segment_recipes_from_metadata(Some(metadata))
                .map_err(AppError::InvalidArgument)?;
            validate_video_segment_recipes_for_plan(
                &previous_recipes,
                &previous_segments,
                previous_style,
            )
            .map_err(AppError::InvalidArgument)?;
            let source = serde_json::from_value(
                metadata["video_execution_identity"]["shooting_script_identity"].clone(),
            )
            .map_err(|error| {
                AppError::InvalidArgument(format!("已有片段规划的拍摄剧本来源无效: {error}"))
            })?;
            Ok((previous_segments, previous_recipes, source))
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
        service.database.executor(),
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
    a4n_workflow_core::update_video_segment_plan_draft_recipe_checkpoint(
        service.database.executor(),
        workflow_run_id,
        source_shot_id,
        segment_index_in_shot,
        style,
        recipe,
    )
    .await
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
    let checkpoints = checkpoints
        .iter()
        .map(
            |checkpoint| a4n_video_contract::VideoSegmentPlanDraftCheckpoint {
                step_key: checkpoint.step_key.clone(),
                extra_json: checkpoint.extra_json.clone(),
                parsed_output_json: checkpoint.parsed_output_json.clone(),
                artifacts_json: checkpoint.artifacts_json.clone(),
            },
        )
        .collect::<Vec<_>>();
    a4n_video_contract::load_video_segment_plan_draft_recipe_state(&checkpoints, style)
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

pub(in crate::video::prompts::generation::frame_prompts) fn select_video_segment_recipe_state_by_shots(
    segments: Vec<VideoSegmentPlan>,
    recipes: Vec<VideoSegmentExecutionRecipeV1>,
    included_source_shot_ids: &HashSet<String>,
    style: &str,
) -> Result<(Vec<VideoSegmentPlan>, Vec<VideoSegmentExecutionRecipeV1>), AppError> {
    let recipes_by_number = recipes.iter().fold(
        HashMap::with_capacity(recipes.len()),
        |mut by_number, recipe| {
            by_number.entry(recipe.segment_number).or_insert(recipe);
            by_number
        },
    );
    let mut selected = segments
        .into_iter()
        .filter(|segment| included_source_shot_ids.contains(&segment.source_shot_id))
        .map(|segment| {
            let recipe = recipes_by_number
                .get(&segment.segment_number)
                .copied()
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_video_contract::{VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1};

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
