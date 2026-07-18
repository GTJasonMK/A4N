use super::super::{contracts::ChapterGenerationProgress, progress::report_progress};
use crate::AppState;
use a4n_core::{error::AppError, models::chapter::chapter};

pub(super) async fn finish_chapter_generation_workflow(
    state: &AppState,
    progress: &mut Option<ChapterGenerationProgress<'_>>,
    progress_total: u32,
    task_id: String,
    workflow_run_id: i64,
    result: Result<chapter::Model, AppError>,
) -> Result<String, String> {
    match result {
        Ok(chapter) => {
            report_progress(progress, 1, progress_total, "正文已保存")?;
            let postprocess_result =
                crate::workflow::postprocess_chapter_after_content_change(state, chapter.id)
                    .await
                    .map_err(|error| error.to_string());
            match postprocess_result {
                Ok(()) => {
                    state
                        .orchestrator
                        .complete_chapter_generation_workflow(workflow_run_id)
                        .await?;
                    Ok(task_id)
                }
                Err(error) => {
                    state
                        .orchestrator
                        .fail_chapter_generation_workflow(workflow_run_id, &error)
                        .await
                        .map_err(|status_error| {
                            format!("{error}; 记录章节生成工作流失败状态失败: {status_error}")
                        })?;
                    Err(error)
                }
            }
        }
        Err(error) => {
            let message = error.to_string();
            state
                .orchestrator
                .fail_chapter_generation_workflow(workflow_run_id, &message)
                .await
                .map_err(|status_error| {
                    format!("{message}; 记录章节生成工作流失败状态失败: {status_error}")
                })?;
            Err(message)
        }
    }
}
