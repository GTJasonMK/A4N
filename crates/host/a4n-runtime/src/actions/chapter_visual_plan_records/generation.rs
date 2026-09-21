use crate::AppState;
use a4n_api_contract::{
    chapter_visual_plan::*,
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
};
use a4n_error::AppError;

pub async fn preview_visual_plan_point_generation(
    state: &AppState,
    user_id: i64,
    input: PreviewVisualPlanPointInput,
) -> Result<VisualPlanPointGenerationPreview, AppError> {
    state
        .chapter_visual_plan_read_application()
        .preview_point_generation(user_id, input)
        .await
}

pub async fn read_visual_plan_prompt_candidate(
    state: &AppState,
    user_id: i64,
    input: ReadVisualPlanPromptCandidateInput,
) -> Result<VisualPlanPromptCandidate, AppError> {
    state
        .chapter_visual_plan_read_application()
        .prompt_candidate(user_id, input)
        .await
}

pub async fn start_visual_plan_prompt_generation(
    state: &AppState,
    user_id: i64,
    input: StartVisualPlanPromptGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskStartError> {
    let start = state
        .chapter_visual_plan_mutation_application()
        .start_prompt_generation(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "director_segment_prompt",
        move |handle| async move {
            worker
                .chapter_visual_plan_mutation_application()
                .run_prompt_generation(user_id, input, handle)
                .await
        },
    );
    Ok(start.handle)
}
