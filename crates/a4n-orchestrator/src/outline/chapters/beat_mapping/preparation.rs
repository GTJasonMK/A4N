use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use serde_json::Value;

use super::super::super::OrchestratorContext;
use super::super::super::chapter_contract::{
    ensure_focus_part_in_context, parse_beat_sheet_json, required_part_summary,
    validate_requested_part_context,
};
use super::super::super::context::{
    ChapterOutlineGenerationContext, load_chapter_outline_generation_context,
};

pub(super) struct PreparedBeatMapping {
    pub(super) beat_payload: Value,
    pub(super) part: part_outline::Model,
    pub(super) count: i32,
    pub(super) context: ChapterOutlineGenerationContext,
}

pub(super) async fn prepare_beat_mapping(
    service: &OrchestratorContext,
    project_id: i64,
    part_id: i64,
    beat_sheet: &str,
) -> Result<PreparedBeatMapping, AppError> {
    let beat_payload = parse_beat_sheet_json(beat_sheet)?;
    let (part, context) = futures::join!(
        service.get_part(part_id),
        load_chapter_outline_generation_context(service.database.connection(), project_id),
    );
    let part = part?;
    if part.project_id != project_id {
        return Err(AppError::InvalidArgument(format!(
            "分部大纲 {part_id} 不属于项目 {project_id}"
        )));
    }
    required_part_summary(&part)?;
    let count = part
        .end_chapter
        .checked_sub(part.start_chapter)
        .and_then(|span| span.checked_add(1))
        .ok_or_else(|| AppError::InvalidArgument("分部章节范围无效".to_string()))?;
    if count <= 0 {
        return Err(AppError::InvalidArgument(
            "分部章节范围必须至少包含 1 章".to_string(),
        ));
    }

    let context = context?;
    validate_requested_part_context(&context.parts, part.start_chapter, part.end_chapter)?;
    ensure_focus_part_in_context(Some(&part), &context.parts)?;

    Ok(PreparedBeatMapping {
        beat_payload,
        part,
        count,
        context,
    })
}
