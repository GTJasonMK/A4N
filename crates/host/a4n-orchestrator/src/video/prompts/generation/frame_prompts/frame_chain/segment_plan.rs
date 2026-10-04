use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
use std::collections::{HashMap, HashSet};

use a4n_video_contract::{
    VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2, validate_video_segment_recipes_for_plan,
};
use serde_json::json;

use crate::video::ChapterVideoPromptStageInput;
use crate::video::prompts::generation::ProgressSpan;
use crate::video::prompts::generation::frame_prompts::{
    PreparedVideoPromptInputs, ReusableSharedSegmentPlanQuery, SHARED_SEGMENT_PLAN_CONTEXT,
    SHARED_SEGMENT_PLAN_CONTRACT_VERSION, SHARED_SEGMENT_PLAN_PROGRESS, SHARED_SEGMENT_PLAN_TITLE,
    VideoPromptChunk, VideoPromptProgress, ensure_no_existing_segment_plan_before_new_planning,
    ensure_segment_plan_generation_can_start, load_previous_shared_segment_recipe_state,
    load_video_segment_plan_draft_checkpoints, load_video_segment_plan_draft_recipe_state,
    report_video_prompt_progress, reusable_shared_segment_plan,
    select_video_segment_recipe_state_by_shots, video_execution_chain_identity,
    video_segment_recipes_from_draft_for_final_plan,
};
use crate::video::prompts::store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode,
    VideoPromptGenerationTrace, new_prompt_draft,
};

use super::contracts::{
    SegmentPlanFromShootingScriptProjectionInput, SharedSegmentPlanInput,
    ShootingScriptProjectionPreparationInput,
};
use super::shooting_script_projection::render_segment_plan_from_shooting_script_projection;

pub(in crate::video::prompts::generation::frame_prompts) struct SegmentPlanDraftInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub source_metadata: &'a serde_json::Value,
    pub stage_outputs: serde_json::Value,
    pub execution_identity: serde_json::Value,
    pub segments: Vec<VideoSegmentPlanV2>,
    pub recipes: Vec<VideoSegmentExecutionRecipeV1>,
    pub plan_complete: bool,
    pub generation_trace: Option<VideoPromptGenerationTrace>,
}

pub(in crate::video::prompts::generation::frame_prompts) fn segment_plan_draft(
    input: SegmentPlanDraftInput<'_>,
) -> NewChapterVideoPrompt {
    let canonical_segment_out = json!({ "segments": &input.segments }).to_string();
    new_prompt_draft(NewChapterVideoPromptInput {
        project_id: input.project_id,
        chapter_id: input.chapter_id,
        workflow_mode: PromptWorkflowMode::SharedSegmentPlan,
        style: Some(input.style),
        prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        shot_number: 0,
        title: Some(SHARED_SEGMENT_PLAN_TITLE),
        scene_summary: None,
        visual_prompt: &canonical_segment_out,
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: input.duration_secs,
        aspect_ratio: Some(input.aspect_ratio),
        reference_image_ids_json: Some(json!([])),
        metadata_json: Some(json!({
            "contract_version": SHARED_SEGMENT_PLAN_CONTRACT_VERSION,
            "video_execution_identity": input.execution_identity,
            "stage_outputs": input.stage_outputs,
            "segments": input.segments,
            "segment_recipes": input.recipes,
            "plan_complete": input.plan_complete,
            "source": input.source_metadata
        })),
    })
    .with_generation_trace_if(input.generation_trace)
}

pub(in crate::video::prompts::generation::frame_prompts) async fn generate_shared_segment_plan(
    input: SharedSegmentPlanInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
    let SharedSegmentPlanInput { service, stage } = input;
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    } = stage;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        project_context,
        vars,
        workflow_run_id,
        target_narrative_unit_index,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;

    let segment_storage_workflow_mode = PromptWorkflowMode::SharedSegmentPlan;
    let segment_storage_prompt_type = chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN;
    let segment_storage_context = SHARED_SEGMENT_PLAN_CONTEXT;
    let segment_storage_progress = SHARED_SEGMENT_PLAN_PROGRESS;
    let incremental_unit = target_narrative_unit_index.is_some();
    let reusable_segment = if regenerate_existing || incremental_unit {
        None
    } else {
        reusable_shared_segment_plan(ReusableSharedSegmentPlanQuery {
            service,
            project_id,
            chapter_id,
            source_metadata: &source_metadata,
            source_spans: &source_spans,
            style: &style,
            duration_secs,
            aspect_ratio: &aspect_ratio,
        })
        .await?
    };
    let previous_recipe_state = load_previous_shared_segment_recipe_state(
        service,
        project_id,
        chapter_id,
        regenerate_existing
            || incremental_unit
            || reusable_segment
                .as_ref()
                .is_some_and(|plan| !plan.plan_complete),
    )
    .await?;
    if !regenerate_existing && !incremental_unit && reusable_segment.is_none() {
        ensure_no_existing_segment_plan_before_new_planning(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            segment_storage_context,
        )
        .await?;
        ensure_segment_plan_generation_can_start(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    }

    report_video_prompt_progress(&mut progress, 20, 100, segment_storage_progress)?;
    // 合法的部分规划继续走逐调用恢复；只有完整规划可以直接结束规划阶段。
    let segment_draft = if let Some(existing) = reusable_segment.filter(|plan| plan.plan_complete) {
        existing.draft
    } else {
        let prepared = render_segment_plan_from_shooting_script_projection(
            SegmentPlanFromShootingScriptProjectionInput {
                shooting_script_projection: ShootingScriptProjectionPreparationInput {
                    service,
                    project_id,
                    chapter_id,
                    source,
                    source_metadata: &source_metadata,
                    source_spans: &source_spans,
                    project_context: project_context.as_ref(),
                    style: &style,
                    prompt_output_language: prompt_output_language.as_str(),
                    previous_segment_plan: previous_recipe_state
                        .as_ref()
                        .map(|(segments, _, _)| segments.as_slice()),
                    previous_segment_recipes: previous_recipe_state
                        .as_ref()
                        .map(|(_, recipes, _)| recipes.as_slice()),
                    previous_script_identity: previous_recipe_state
                        .as_ref()
                        .map(|(_, _, source)| source),
                    reuse_published_shots: !regenerate_existing,
                    duration_secs,
                    aspect_ratio: &aspect_ratio,
                    prompt_renderer,
                    variables: &vars,
                    workflow_run_id,
                    target_narrative_unit_index,
                    progress_span: ProgressSpan::new(20, 45),
                    cancel_token,
                },
                segment_context: segment_storage_context,
                segment_progress: segment_storage_progress,
                segment_span: ProgressSpan::new(45, 90),
            },
            &mut progress,
            &mut chunks,
        )
        .await?;
        if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
            return Err(AppError::Cancelled);
        }
        let new_segment_plan = prepared.segment_plan;
        let updated_source_shots = new_segment_plan
            .iter()
            .map(|segment| segment.source_shot_id.clone())
            .collect::<HashSet<_>>();
        let (segment_plan, segment_recipes) = if new_segment_plan.is_empty() {
            let previous =
                previous_recipe_state
                    .as_ref()
                    .ok_or_else(|| AppError::InvalidState {
                        current: "missing_previous_segment_plan".to_string(),
                        operation: "复用已完成场景的片段规划".to_string(),
                    })?;
            merge_incremental_segment_plan(
                Some((&previous.0, &previous.1)),
                Vec::new(),
                Vec::new(),
                &style,
                &prepared.source_shot_order,
            )?
        } else {
            let workflow_run_id = workflow_run_id.ok_or_else(|| {
                AppError::InvalidArgument("视频片段规划缺少 workflow_run_id".to_string())
            })?;
            let draft_checkpoints =
                load_video_segment_plan_draft_checkpoints(service, workflow_run_id).await?;
            let (draft_segments, draft_recipes, _, _) =
                load_video_segment_plan_draft_recipe_state(&draft_checkpoints, &style)?;
            let new_source_shot_ids = new_segment_plan
                .iter()
                .map(|segment| segment.source_shot_id.clone())
                .collect::<HashSet<_>>();
            let (draft_segments, draft_recipes) = select_video_segment_recipe_state_by_shots(
                draft_segments,
                draft_recipes,
                &new_source_shot_ids,
                &style,
            )?;
            let new_segment_recipes = video_segment_recipes_from_draft_for_final_plan(
                &draft_segments,
                &draft_recipes,
                &new_segment_plan,
                &style,
                prompt_output_language.as_str(),
            )?;
            if previous_recipe_state.is_some() {
                merge_incremental_segment_plan(
                    previous_recipe_state
                        .as_ref()
                        .map(|(segments, recipes, _)| (segments.as_slice(), recipes.as_slice())),
                    new_segment_plan,
                    new_segment_recipes,
                    &style,
                    &prepared.source_shot_order,
                )?
            } else {
                (new_segment_plan, new_segment_recipes)
            }
        };
        let plan_complete = prepared.source_complete
            && segment_plan_covers_required_shots(&segment_plan, &prepared.source_shot_order);
        let mut execution_identity = video_execution_chain_identity(
            &prepared.shooting_script_projection,
            &segment_plan,
            &style,
            duration_secs,
            &aspect_ratio,
        )?;
        super::super::checkpoint::carry_forward_execution_sources(
            &mut execution_identity,
            previous_recipe_state.as_ref().map(|(_, _, source)| source),
            &updated_source_shots,
        )?;
        segment_plan_draft(SegmentPlanDraftInput {
            project_id,
            chapter_id,
            style: &style,
            duration_secs,
            aspect_ratio: &aspect_ratio,
            source_metadata: &source_metadata,
            stage_outputs: prepared.segment_stage_outputs,
            execution_identity,
            segments: segment_plan,
            recipes: segment_recipes,
            plan_complete,
            generation_trace: prepared.segment_trace,
        })
    };
    if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    let rows = service
        .sync_chapter_prompts(chapter_id, vec![segment_draft])
        .await?;
    report_video_prompt_progress(&mut progress, 100, 100, "视频片段规划记录已更新")?;
    Ok(rows)
}

pub(in crate::video::prompts::generation::frame_prompts) fn segment_plan_covers_required_shots(
    segments: &[VideoSegmentPlanV2],
    required_source_shot_ids: &[String],
) -> bool {
    let planned_shot_ids = segments
        .iter()
        .map(|segment| segment.source_shot_id.as_str())
        .collect::<HashSet<_>>();
    !required_source_shot_ids.is_empty()
        && required_source_shot_ids
            .iter()
            .all(|shot_id| planned_shot_ids.contains(shot_id.as_str()))
}

pub(in crate::video::prompts::generation::frame_prompts) fn merge_incremental_segment_plan(
    previous: Option<(&[VideoSegmentPlanV2], &[VideoSegmentExecutionRecipeV1])>,
    new_segments: Vec<VideoSegmentPlanV2>,
    new_recipes: Vec<VideoSegmentExecutionRecipeV1>,
    style: &str,
    source_shot_order: &[String],
) -> Result<(Vec<VideoSegmentPlanV2>, Vec<VideoSegmentExecutionRecipeV1>), AppError> {
    let replaced_shots = new_segments
        .iter()
        .map(|segment| segment.source_shot_id.clone())
        .collect::<HashSet<_>>();
    let new_recipes_by_number = new_recipes.iter().fold(
        HashMap::with_capacity(new_recipes.len()),
        |mut by_number, recipe| {
            by_number.entry(recipe.segment_number).or_insert(recipe);
            by_number
        },
    );
    let previous_recipes_by_number = previous.map(|(_, recipes)| {
        recipes.iter().fold(
            HashMap::with_capacity(recipes.len()),
            |mut by_number, recipe| {
                by_number.entry(recipe.segment_number).or_insert(recipe);
                by_number
            },
        )
    });
    let new_pairs = new_segments
        .into_iter()
        .map(|segment| {
            let recipe = new_recipes_by_number
                .get(&segment.segment_number)
                .copied()
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "新片段 {} 缺少执行方案",
                        segment.segment_number
                    ))
                })?;
            Ok((segment, recipe))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let mut pairs = Vec::new();
    let mut inserted_new = false;
    if let Some((previous_segments, _)) = previous {
        for segment in previous_segments {
            if !source_shot_order.contains(&segment.source_shot_id) {
                continue;
            }
            if replaced_shots.contains(segment.source_shot_id.as_str()) {
                if !inserted_new {
                    pairs.extend(new_pairs.iter().cloned());
                    inserted_new = true;
                }
                continue;
            }
            let recipe = previous_recipes_by_number
                .as_ref()
                .and_then(|by_number| by_number.get(&segment.segment_number).copied())
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已有片段 {} 缺少执行方案",
                        segment.segment_number
                    ))
                })?;
            pairs.push((segment.clone(), recipe));
        }
    }
    if !inserted_new {
        pairs.extend(new_pairs);
    }
    let source_order = source_shot_order
        .iter()
        .enumerate()
        .map(|(index, shot_id)| (shot_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    for (segment, _) in &pairs {
        if !source_order.contains_key(segment.source_shot_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "已有片段引用了当前拍摄剧本不存在的镜头 {}",
                segment.source_shot_id
            )));
        }
    }
    pairs.sort_by_key(|(segment, _)| {
        (
            source_order[segment.source_shot_id.as_str()],
            segment.segment_index_in_shot,
        )
    });
    for (offset, (segment, recipe)) in pairs.iter_mut().enumerate() {
        let segment_number = i32::try_from(offset + 1)
            .map_err(|_| AppError::InvalidArgument("片段数量超出 i32 范围".to_string()))?;
        segment.segment_number = segment_number;
        recipe.segment_number = segment_number;
    }
    let (segments, recipes): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
    validate_video_segment_recipes_for_plan(&recipes, &segments, style)
        .map_err(AppError::InvalidArgument)?;
    Ok((segments, recipes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::prompts::generation::frame_prompts::{
        tests::{segment, shooting_script_projection},
        video_segment_execution_identity,
    };
    use a4n_video_contract::default_video_segment_recipes;

    #[test]
    fn incremental_plan_appends_new_shots_and_replaces_existing_shots_in_place() {
        let previous_segments = vec![segment(1), segment(2)];
        let previous_recipes =
            default_video_segment_recipes(&previous_segments, "cinematic", "zh_cn").unwrap();

        let appended_segment = segment(3);
        let appended_recipes = default_video_segment_recipes(
            std::slice::from_ref(&appended_segment),
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        let (appended, appended_recipes) = merge_incremental_segment_plan(
            Some((&previous_segments, &previous_recipes)),
            vec![appended_segment],
            appended_recipes,
            "cinematic",
            &[
                "shot-1".to_string(),
                "shot-2".to_string(),
                "shot-3".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(
            appended
                .iter()
                .map(|segment| segment.source_shot_id.as_str())
                .collect::<Vec<_>>(),
            vec!["shot-1", "shot-2", "shot-3"]
        );
        let final_identity = video_execution_chain_identity(
            &shooting_script_projection(),
            &appended,
            "cinematic",
            5,
            "16:9",
        )
        .unwrap();
        for (source_shot_id, segment_number) in [("shot-1", 1), ("shot-3", 3)] {
            let recipe = appended_recipes
                .iter()
                .find(|recipe| recipe.segment_number == segment_number);
            video_segment_execution_identity(
                final_identity.clone(),
                Some(7),
                recipe,
                (source_shot_id, 1),
            )
            .unwrap();
        }
        let required = vec![
            "shot-1".to_string(),
            "shot-2".to_string(),
            "shot-3".to_string(),
        ];
        assert!(segment_plan_covers_required_shots(&appended, &required));
        assert!(!segment_plan_covers_required_shots(&appended, &[]));
        assert!(!segment_plan_covers_required_shots(
            &appended,
            &["shot-4".to_string()]
        ));

        let unordered_previous = vec![segment(1), segment(3)];
        let unordered_recipes =
            default_video_segment_recipes(&unordered_previous, "cinematic", "zh_cn").unwrap();
        let middle_segment = segment(2);
        let middle_recipes = default_video_segment_recipes(
            std::slice::from_ref(&middle_segment),
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        let (ordered, _) = merge_incremental_segment_plan(
            Some((&unordered_previous, &unordered_recipes)),
            vec![middle_segment],
            middle_recipes,
            "cinematic",
            &required,
        )
        .unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|segment| segment.source_shot_id.as_str())
                .collect::<Vec<_>>(),
            vec!["shot-1", "shot-2", "shot-3"]
        );

        let mut replacement = segment(9);
        replacement.source_shot_id = "shot-1".to_string();
        replacement.title = "replacement".to_string();
        let replacement_recipes =
            default_video_segment_recipes(std::slice::from_ref(&replacement), "cinematic", "zh_cn")
                .unwrap();
        let (replaced, _) = merge_incremental_segment_plan(
            Some((&previous_segments, &previous_recipes)),
            vec![replacement],
            replacement_recipes,
            "cinematic",
            &["shot-1".to_string(), "shot-2".to_string()],
        )
        .unwrap();
        assert_eq!(replaced[0].source_shot_id, "shot-1");
        assert_eq!(replaced[0].title, "replacement");
        assert_eq!(replaced[1].source_shot_id, "shot-2");
        let (remaining, _) = merge_incremental_segment_plan(
            Some((&previous_segments, &previous_recipes)),
            Vec::new(),
            Vec::new(),
            "cinematic",
            &["shot-2".to_string()],
        )
        .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].source_shot_id, "shot-2");
        assert_eq!(remaining[0].segment_number, 1);
        assert_eq!(
            a4n_video_contract::video_segment_content_fingerprint(&remaining[0]).unwrap(),
            a4n_video_contract::video_segment_content_fingerprint(&previous_segments[1]).unwrap()
        );

        let unknown = segment(3);
        let unknown_recipes =
            default_video_segment_recipes(std::slice::from_ref(&unknown), "cinematic", "zh_cn")
                .unwrap();
        assert!(
            merge_incremental_segment_plan(
                None,
                vec![unknown],
                unknown_recipes,
                "cinematic",
                &["shot-1".to_string()],
            )
            .unwrap_err()
            .to_string()
            .contains("当前拍摄剧本不存在")
        );
    }
}
