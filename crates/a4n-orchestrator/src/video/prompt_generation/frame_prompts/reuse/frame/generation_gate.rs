use a4n_core::error::AppError;
use a4n_core::prompt_language::metadata_matches_prompt_output_language;

use crate::video::prompt_store::PromptWorkflowMode;
use crate::video::{OrchestratorContext, PromptOutputLanguage};

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_video_prompt_generation_can_start(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    shot_number: i32,
    label: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let rows = a4n_db::video::chapter_video_prompt_workflow_shot_records(
        service.db.connection(),
        project_id,
        chapter_id,
        workflow_mode.as_str(),
        "video_prompt",
        shot_number,
    )
    .await
    .map_err(AppError::Database)?;
    for row in rows {
        if !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        ) {
            continue;
        }
        if row.status == "running" {
            return Err(AppError::InvalidArgument(format!(
                "当前章节已有{label}正在运行，不能重新生成视频提示词"
            )));
        }
        service
            .ensure_prompt_stage_mutable(&row, "重新生成")
            .await?;
    }
    Ok(())
}
