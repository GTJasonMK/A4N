use crate::commands::request_preview::ModelRequestPreview;
use a4n_core::event::sse::{ProgressMeta, ProgressPhaseKind, ProgressTaskKind};
use a4n_runtime::AppState;
use a4n_runtime::actions::manga_pipeline as manga_pipeline_actions;
use a4n_services::manga::MangaPipelineStartStage;
use tauri::{State, Window};

#[tauri::command]
pub async fn start_manga_pipeline(
    state: State<'_, AppState>,
    window: Window,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    start_from_stage: Option<String>,
) -> Result<String, String> {
    let start_from_stage = parse_manga_pipeline_start_stage(start_from_stage)?;
    spawn_manga_pipeline(
        state,
        window,
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
    window: Window,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<String, String> {
    spawn_manga_pipeline(
        state,
        window,
        user_id,
        project_id,
        chapter_number,
        true,
        None,
    )
    .await
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
    window: Window,
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
    let bridge = std::sync::Arc::new(crate::events::bridge::EventBridge::new(window));
    let start_message = if resume_from_checkpoint {
        "续传漫画 8 步流水线"
    } else {
        "启动漫画 8 步流水线"
    };
    let _ = bridge.emit_progress_with_meta(
        &task_id,
        0,
        8,
        start_message,
        Some(manga_progress_meta(
            "manga_pipeline_start",
            "漫画流水线",
            start_message,
        )),
    );

    let state_clone: AppState = (*state.inner()).clone();
    let bridge_clone = bridge.clone();
    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        let _ = bridge_clone.emit_progress_with_meta(
            &task_id_clone,
            1,
            8,
            "正在运行 LLM 流水线",
            Some(manga_progress_meta(
                "manga_pipeline_llm",
                "漫画 LLM 流水线",
                "正在运行 LLM 流水线",
            )),
        );
        let result = manga_pipeline_actions::run_manga_pipeline(
            &state_clone,
            user_id,
            project_id,
            chapter_number,
            &task_id_clone,
            resume_from_checkpoint,
            start_from_stage,
        )
        .await;
        match result {
            Ok(panels) => {
                let _ = bridge_clone.emit_complete(
                    &task_id_clone,
                    Some(format!(
                        "漫画流水线完成，生成 {} 页整页提示词",
                        panels.len()
                    )),
                );
            }
            Err(err) => {
                let _ = bridge_clone.emit_error(
                    &task_id_clone,
                    "pipeline_failed",
                    &err.to_string(),
                    false,
                );
            }
        }
    });

    Ok(task_id)
}

fn manga_progress_meta(stage_code: &str, stage_label: &str, detail: &str) -> ProgressMeta {
    ProgressMeta::new(
        ProgressTaskKind::MangaGeneration,
        stage_code,
        stage_label,
        ProgressPhaseKind::ModelStream,
    )
    .with_detail(detail)
}

fn parse_manga_pipeline_start_stage(
    value: Option<String>,
) -> Result<Option<MangaPipelineStartStage>, String> {
    manga_pipeline_actions::parse_manga_pipeline_start_stage(value).map_err(|err| err.to_string())
}
