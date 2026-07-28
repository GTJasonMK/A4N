use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::manga_pipeline::{
    self as manga_pipeline_actions, MangaPipelineStartStage,
};
use tauri::State;

#[tauri::command]
pub async fn start_manga_pipeline(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    start_from_stage: Option<String>,
) -> Result<String, String> {
    let start_from_stage = parse_manga_pipeline_start_stage(start_from_stage)?;
    spawn_manga_pipeline(
        state,
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
}

#[tauri::command]
pub async fn resume_manga_pipeline(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<String, String> {
    spawn_manga_pipeline(state, user_id, project_id, chapter_number, true, None).await
}

#[tauri::command]
pub async fn preview_start_manga_pipeline_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    start_from_stage: Option<String>,
) -> Result<ModelRequestPreview, String> {
    let start_from_stage = parse_manga_pipeline_start_stage(start_from_stage)?;
    manga_pipeline_actions::preview_manga_pipeline_requests(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn preview_resume_manga_pipeline_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<ModelRequestPreview, String> {
    manga_pipeline_actions::preview_manga_pipeline_requests(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        true,
        None,
    )
    .await
    .map_err(|err| err.to_string())
}

async fn spawn_manga_pipeline(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<String, String> {
    manga_pipeline_actions::preflight_manga_pipeline(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
    )
    .await
    .map_err(|err| err.to_string())?;
    let task_id = format!(
        "manga-pipeline-{project_id}-{chapter_number}-{}",
        chrono::Utc::now().timestamp_millis()
    );

    let state_clone: AppState = (*state.inner()).clone();
    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        if let Err(error) = manga_pipeline_actions::run_manga_pipeline(
            &state_clone,
            user_id,
            project_id,
            chapter_number,
            &task_id_clone,
            resume_from_checkpoint,
            start_from_stage,
        )
        .await
        {
            tracing::error!(
                task_id = task_id_clone,
                project_id,
                chapter_number,
                %error,
                "漫画流水线执行失败"
            );
        }
    });

    Ok(task_id)
}

fn parse_manga_pipeline_start_stage(
    value: Option<String>,
) -> Result<Option<MangaPipelineStartStage>, String> {
    manga_pipeline_actions::parse_manga_pipeline_start_stage(value).map_err(|err| err.to_string())
}
