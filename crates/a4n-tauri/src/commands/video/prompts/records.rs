use a4n_runtime::AppState;
use a4n_runtime::host_api::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::host_api::video_prompts::{
    self as video_prompt_actions, ChapterVideoPromptSummary, LoadVideoSegmentPlanDraftInput,
    OptimizeChapterVideoPromptInput, UpdateChapterVideoPromptInput,
    UpdateVideoSegmentPlanDraftRecipeInput, VideoSegmentExecutionRecipeV1,
    VideoSegmentPlanDraftSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_chapter_video_prompts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    video_prompt_actions::list_chapter_video_prompts(state.inner(), user_id, project_id, chapter_id)
        .await
}

#[tauri::command]
pub async fn load_video_segment_plan_draft(
    state: State<'_, AppState>,
    user_id: i64,
    input: LoadVideoSegmentPlanDraftInput,
) -> Result<VideoSegmentPlanDraftSummary, String> {
    video_prompt_actions::load_video_segment_plan_draft(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn update_video_segment_plan_draft_recipe(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateVideoSegmentPlanDraftRecipeInput,
) -> Result<(), String> {
    video_prompt_actions::update_video_segment_plan_draft_recipe(state.inner(), user_id, input)
        .await
}

#[tauri::command]
pub async fn update_chapter_video_segment_recipe(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    video_prompt_actions::update_chapter_video_segment_recipe(
        state.inner(),
        user_id,
        prompt_id,
        recipe,
    )
    .await
}

#[tauri::command]
pub async fn update_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    video_prompt_actions::update_chapter_video_prompt(state.inner(), user_id, prompt_id, input)
        .await
}

#[tauri::command]
pub async fn optimize_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    video_prompt_actions::optimize_chapter_video_prompt(state.inner(), user_id, prompt_id, input)
        .await
}

#[tauri::command]
pub async fn delete_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: Option<String>,
) -> Result<(), String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    video_prompt_actions::delete_chapter_video_prompt(
        state.inner(),
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
}
