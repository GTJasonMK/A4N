use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_error::AppError;

use super::super::contracts::ReusableFrameRowsQuery;
use crate::video::prompts::records::PromptWorkflowRowsContext;

pub(in crate::video::prompts::generation::frame_prompts) async fn reusable_frame_rows_for_resume(
    query: ReusableFrameRowsQuery<'_>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let ReusableFrameRowsQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        source_metadata,
        style,
        duration_secs,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint,
        source_prompt_type,
        video_execution_identity,
        prompt_output_language,
    } = query;
    service
        .reusable_prompt_rows(
            PromptWorkflowRowsContext {
                project_id,
                chapter_id,
                workflow_mode: workflow_mode.as_str(),
                prompt_type,
                source_metadata,
                style,
                duration_secs,
                aspect_ratio,
                reference_ids,
                generation_asset_fingerprint: Some(generation_asset_fingerprint),
                source_prompt_type: Some(source_prompt_type),
                video_execution_identity,
            },
            prompt_output_language,
        )
        .await
}
