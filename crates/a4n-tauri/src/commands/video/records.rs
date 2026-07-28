use a4n_api_contract::video::SelectSegmentVideoInput;
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_generation::GeneratedVideoSummary;
use a4n_runtime::host_api::video_records::{
    self as video_records_actions, ApplyAssemblyPostprocessEditInput, AssembledChapterVideoPreview,
    AssemblyPushResult, DeleteChapterVideoAssetsResult, PushAssembledChapterVideoInput,
    UpdateAssemblyPushStatusInput, UpdateGeneratedVideoInput, UploadAssemblyInsertVideoInput,
    UploadChapterVideoInput, UploadReferenceMediaVideoInput, VideoPlaybackData,
};
use tauri::State;

#[tauri::command]
pub async fn list_videos_by_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoSummary>, String> {
    video_records_actions::list_videos_by_chapter(state.inner(), user_id, project_id, chapter_id)
        .await
}

#[tauri::command]
pub async fn toggle_video_favorite(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::toggle_video_favorite(state.inner(), user_id, video_id).await
}

#[tauri::command]
pub async fn update_video(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
    input: UpdateGeneratedVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::update_video(state.inner(), user_id, video_id, input).await
}

#[tauri::command]
pub async fn select_segment_video(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    input: SelectSegmentVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::select_segment_video(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        video_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn load_video_playback_data(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    video_records_actions::load_video_playback_data(state.inner(), user_id, video_id).await
}

#[tauri::command]
pub async fn assemble_chapter_videos(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
) -> Result<AssembledChapterVideoPreview, String> {
    video_records_actions::assemble_chapter_videos(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
}

#[tauri::command]
pub async fn load_cached_assembled_chapter_video_preview(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
) -> Result<Option<AssembledChapterVideoPreview>, String> {
    video_records_actions::load_cached_assembled_chapter_video_preview(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
}

#[tauri::command]
pub async fn push_assembled_chapter_video(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: PushAssembledChapterVideoInput,
) -> Result<AssemblyPushResult, String> {
    video_records_actions::push_assembled_chapter_video(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
}

#[tauri::command]
pub async fn update_assembly_push_status(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
    input: UpdateAssemblyPushStatusInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::update_assembly_push_status(state.inner(), user_id, video_id, input)
        .await
}

#[tauri::command]
pub async fn apply_assembly_postprocess_edit(
    state: State<'_, AppState>,
    user_id: i64,
    push_video_id: i64,
    input: ApplyAssemblyPostprocessEditInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::apply_assembly_postprocess_edit(
        state.inner(),
        user_id,
        push_video_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn delete_video(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
) -> Result<(), String> {
    video_records_actions::delete_video(state.inner(), user_id, video_id).await
}

#[tauri::command]
pub async fn delete_chapter_video_assets(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<DeleteChapterVideoAssetsResult, String> {
    video_records_actions::delete_chapter_video_assets(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
}

#[tauri::command]
pub async fn upload_chapter_video(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::upload_chapter_video(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn upload_reference_media_video(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::upload_reference_media_video(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn upload_assembly_insert_video(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: UploadAssemblyInsertVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::upload_assembly_insert_video(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
}

#[tauri::command]
pub async fn trim_video(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    video_records_actions::trim_video(state.inner(), user_id, video_id, start_secs, end_secs).await
}
