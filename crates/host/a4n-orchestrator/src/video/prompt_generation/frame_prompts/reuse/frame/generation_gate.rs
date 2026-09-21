use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::prompt_language::{PromptOutputLanguage, metadata_matches_prompt_output_language};
use a4n_error::AppError;

use crate::video::OrchestratorContext;
use crate::video::prompt_store::PromptWorkflowMode;

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
        service.database.connection(),
        project_id,
        chapter_id,
        workflow_mode.as_str(),
        chapter_video_prompt::PROMPT_TYPE_VIDEO,
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
        if row.status == chapter_video_prompt::STATUS_RUNNING {
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
