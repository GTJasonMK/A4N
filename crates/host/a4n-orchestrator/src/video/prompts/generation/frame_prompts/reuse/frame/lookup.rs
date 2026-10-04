use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use std::collections::BTreeMap;

use a4n_domain::prompt_language::metadata_matches_prompt_output_language;
use a4n_error::AppError;

use super::super::super::prompt_resume_context_matches;
use super::super::contracts::{FrameImageLookupContext, FrameImageLookupRowsQuery};

pub(in crate::video) async fn boundary_frame_rows_for_image_lookup(
    query: FrameImageLookupRowsQuery<'_>,
) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
    let FrameImageLookupRowsQuery {
        service,
        project_id,
        chapter_id,
        context,
    } = query;
    let rows = a4n_db::video::read_models::chapter_video_prompt_workflow_records(
        service.database.executor(),
        project_id,
        chapter_id,
        context.prompt_type,
        context.workflow_mode.as_str(),
        true,
    )
    .await
    .map_err(AppError::from)?;

    let mut by_frame = BTreeMap::<i32, ChapterVideoPromptRecord>::new();
    for row in rows {
        if !frame_row_matches_image_lookup_context(&row, &context) {
            continue;
        }
        by_frame.entry(row.shot_number).or_insert(row);
    }
    Ok(by_frame.into_values().collect())
}

pub(in crate::video::prompts::generation::frame_prompts) fn frame_row_matches_image_lookup_context(
    row: &ChapterVideoPromptRecord,
    context: &FrameImageLookupContext<'_>,
) -> bool {
    prompt_resume_context_matches(
        row,
        context.source_metadata,
        context.style,
        context.aspect_ratio,
    ) && metadata_matches_prompt_output_language(
        row.metadata_json.as_ref(),
        context.prompt_output_language,
    ) && super::inputs::frame_prompt_row_matches_video_input_contract(
        row,
        context.workflow_mode,
        context.prompt_type,
        context.source_prompt_type,
        context.video_execution_identity,
    )
}
