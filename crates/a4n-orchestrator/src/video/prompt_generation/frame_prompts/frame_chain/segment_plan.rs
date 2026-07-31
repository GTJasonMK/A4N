use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::json;

use crate::video::ChapterVideoPromptStageInput;
use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::{
    PreparedVideoPromptInputs, ReusableSharedSegmentPlanQuery, SHARED_SEGMENT_PLAN_CONTEXT,
    SHARED_SEGMENT_PLAN_CONTRACT_VERSION, SHARED_SEGMENT_PLAN_PROGRESS,
    SHARED_SEGMENT_PLAN_PROMPT_TYPE, SHARED_SEGMENT_PLAN_TITLE, VideoPromptChunk,
    VideoPromptProgress, ensure_existing_segment_plans_can_regenerate,
    ensure_no_existing_segment_plan_before_new_planning, ensure_segment_plan_generation_can_start,
    load_previous_shared_segment_recipe_state, load_video_segment_plan_draft_recipe_state,
    materialize_video_segment_plan_draft_recipes, report_video_prompt_progress,
    reusable_shared_segment_plan, video_segment_recipes_from_draft_for_final_plan,
};
use crate::video::prompt_store::{
    NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};

use super::contracts::{
    SegmentPlanFromShootingScriptProjectionInput, SharedSegmentPlanInput,
    ShootingScriptProjectionPreparationInput,
};
use super::shooting_script_projection::render_segment_plan_from_shooting_script_projection;

pub(in crate::video::prompt_generation::frame_prompts) async fn generate_shared_segment_plan(
    input: SharedSegmentPlanInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
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
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;

    let segment_storage_workflow_mode = PromptWorkflowMode::SharedSegmentPlan;
    let segment_storage_prompt_type = SHARED_SEGMENT_PLAN_PROMPT_TYPE;
    let segment_storage_contract_version = SHARED_SEGMENT_PLAN_CONTRACT_VERSION;
    let segment_storage_context = SHARED_SEGMENT_PLAN_CONTEXT;
    let segment_storage_progress = SHARED_SEGMENT_PLAN_PROGRESS;
    if regenerate_existing {
        ensure_existing_segment_plans_can_regenerate(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    }
    let previous_recipe_state = load_previous_shared_segment_recipe_state(
        service,
        project_id,
        chapter_id,
        regenerate_existing,
    )
    .await?;

    let reusable_segment = if regenerate_existing {
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
    if !regenerate_existing && reusable_segment.is_none() {
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
    let (_segment_plan, segment_draft) = if let Some(existing) = reusable_segment {
        (existing.segment_plan, existing.draft)
    } else {
        let prepared = render_segment_plan_from_shooting_script_projection(
            SegmentPlanFromShootingScriptProjectionInput {
                shooting_script_projection: ShootingScriptProjectionPreparationInput {
                    service,
                    project_id,
                    chapter_id,
                    source,
                    source_spans: &source_spans,
                    project_context: project_context.as_ref(),
                    style: &style,
                    prompt_output_language: prompt_output_language.as_str(),
                    previous_segment_plan: previous_recipe_state
                        .as_ref()
                        .map(|(segments, _)| segments.as_slice()),
                    previous_segment_recipes: previous_recipe_state
                        .as_ref()
                        .map(|(_, recipes)| recipes.as_slice()),
                    duration_secs,
                    aspect_ratio: &aspect_ratio,
                    prompt_renderer,
                    variables: &vars,
                    workflow_run_id,
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
        let canonical_segment_out = prepared.segment_json.to_string();
        let workflow_run_id = workflow_run_id.ok_or_else(|| {
            AppError::InvalidArgument("视频片段规划缺少 workflow_run_id".to_string())
        })?;
        let previous_recipe_refs = previous_recipe_state
            .as_ref()
            .map(|(segments, recipes)| (segments.as_slice(), recipes.as_slice()));
        materialize_video_segment_plan_draft_recipes(
            service,
            workflow_run_id,
            &style,
            prompt_output_language.as_str(),
            previous_recipe_refs,
        )
        .await?;
        let (draft_segments, draft_recipes) =
            load_video_segment_plan_draft_recipe_state(service, workflow_run_id, &style).await?;
        let segment_recipes = video_segment_recipes_from_draft_for_final_plan(
            &draft_segments,
            &draft_recipes,
            &prepared.segment_plan,
            &style,
            prompt_output_language.as_str(),
        )?;
        let segment_draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id,
            chapter_id,
            workflow_mode: segment_storage_workflow_mode,
            style: Some(&style),
            prompt_type: segment_storage_prompt_type,
            shot_number: 0,
            title: Some(SHARED_SEGMENT_PLAN_TITLE),
            scene_summary: None,
            visual_prompt: &canonical_segment_out,
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some(&aspect_ratio),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": segment_storage_contract_version,
                "video_execution_identity": prepared.execution_identity,
                "stage_outputs": prepared.segment_stage_outputs,
                "segments": prepared.segment_plan,
                "segment_recipes": segment_recipes,
                "source": source_metadata.clone()
            })),
        })
        .with_generation_trace_if(prepared.segment_trace);
        (prepared.segment_plan, segment_draft)
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
