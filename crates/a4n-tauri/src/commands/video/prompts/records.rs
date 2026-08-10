use a4n_runtime::AppState;
use a4n_runtime::host_api::video_prompts::{
    self as video_prompt_actions, ChapterVideoPromptSummary, CommitInsertedSegmentDraftInput,
    DeleteChapterVideoPromptInput, DeleteInsertedSegmentInput, GetVideoTimelineInput,
    LoadVideoSegmentPlanDraftInput, MoveInsertedSegmentInput, OptimizeChapterVideoPromptInput,
    UpdateChapterVideoPromptInput, UpdateVideoSegmentPlanDraftRecipeInput,
    VideoSegmentExecutionRecipeV1, VideoSegmentPlanDraftSummary, VideoTimelineSummary,
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
pub async fn get_chapter_video_timeline(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetVideoTimelineInput,
) -> Result<VideoTimelineSummary, String> {
    video_prompt_actions::get_chapter_video_timeline(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn move_inserted_segment(
    state: State<'_, AppState>,
    user_id: i64,
    input: MoveInsertedSegmentInput,
) -> Result<VideoTimelineSummary, String> {
    video_prompt_actions::move_inserted_segment(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn commit_inserted_segment_draft(
    state: State<'_, AppState>,
    user_id: i64,
    input: CommitInsertedSegmentDraftInput,
) -> Result<VideoTimelineSummary, String> {
    video_prompt_actions::commit_inserted_segment_draft(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn delete_inserted_segment(
    state: State<'_, AppState>,
    user_id: i64,
    input: DeleteInsertedSegmentInput,
) -> Result<VideoTimelineSummary, String> {
    video_prompt_actions::delete_inserted_segment(state.inner(), user_id, input).await
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
    input: DeleteChapterVideoPromptInput,
) -> Result<(), String> {
    video_prompt_actions::delete_chapter_video_prompt(state.inner(), user_id, input).await
}
