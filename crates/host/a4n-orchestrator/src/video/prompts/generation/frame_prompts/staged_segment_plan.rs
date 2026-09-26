mod contracts;
mod finalization;
mod recipes;
mod shots;

use a4n_db::workflow;
use a4n_error::AppError;
use std::collections::HashSet;

use crate::workflow::workflow_checkpoint_contract_app_error;

pub(super) use self::contracts::StagedSegmentPlanInput;
use self::contracts::{FinalizeStagedSegmentPlanInput, StagedSegmentPlanResult};
use self::finalization::finalize_staged_segment_plan;
use self::recipes::segment_plan_shot_recipe_artifacts;
pub(super) use self::recipes::{
    load_previous_shared_segment_recipe_state, select_video_segment_recipe_state_by_shots,
};
pub(in crate::video) use self::recipes::{
    load_video_segment_plan_draft_checkpoints, load_video_segment_plan_draft_recipe_state,
    update_video_segment_plan_draft_recipe_checkpoint,
    video_segment_recipes_from_draft_for_final_plan,
};
use self::shots::render_segment_plan_shots;

fn exclude_reused_segment_plan_shots(
    segment_plan: Vec<super::VideoSegmentPlan>,
    reused_source_shot_ids: &[String],
) -> Vec<super::VideoSegmentPlan> {
    let reused_source_shot_ids = reused_source_shot_ids.iter().collect::<HashSet<_>>();
    segment_plan
        .into_iter()
        .filter(|segment| !reused_source_shot_ids.contains(&segment.source_shot_id))
        .collect()
}

pub(super) async fn render_staged_segment_plan_with_duration(
    input: StagedSegmentPlanInput<'_, '_, '_, '_>,
) -> Result<StagedSegmentPlanResult, AppError> {
    let workflow_run_id =
        workflow::required_llm_call_workflow_run_id(input.workflow_run_id, "video_segment_plan")
            .map_err(workflow_checkpoint_contract_app_error)?;
    if input.shooting_script_projection.shot_count() == 0 {
        return Err(AppError::InvalidArgument(
            "视频片段规划必须基于至少一个导演镜头".to_string(),
        ));
    }
    let cancel_token = input.cancel_token;
    let service = input.service;
    let source_spans = input.source_spans;
    let shooting_script_projection = input.shooting_script_projection;
    let duration_secs = input.duration_secs;
    let default_aspect_ratio = input.default_aspect_ratio;
    let rendered_shots = render_segment_plan_shots(input, workflow_run_id).await?;
    finalize_staged_segment_plan(FinalizeStagedSegmentPlanInput {
        service,
        workflow_run_id,
        source_spans,
        shooting_script_projection,
        duration_secs,
        default_aspect_ratio,
        cancel_token,
        rendered_shots,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_shots_are_not_returned_as_new_incremental_output() {
        let segments = vec![
            crate::video::prompts::generation::frame_prompts::tests::segment(1),
            crate::video::prompts::generation::frame_prompts::tests::segment(2),
        ];

        let partial = exclude_reused_segment_plan_shots(segments.clone(), &["shot-1".to_string()]);
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].source_shot_id, "shot-2");

        let all = exclude_reused_segment_plan_shots(
            segments,
            &["shot-1".to_string(), "shot-2".to_string()],
        );
        assert!(all.is_empty());
    }
}
