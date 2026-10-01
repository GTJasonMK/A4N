use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;

use super::super::super::OrchestratorContext;
use super::super::super::context::{PartOutlineGenerationContext, load_part_generation_context};
use super::super::super::part_contract::{
    ensure_no_part_outline_conflict, validate_part_outline_request,
};

pub(super) struct PreparedPartRegeneration {
    pub(super) project_id: i64,
    pub(super) target: PartOutlineRecord,
    pub(super) context: PartOutlineGenerationContext,
    pub(super) previous_parts: Vec<PartOutlineRecord>,
    pub(super) rebind_chapter_outlines: bool,
}

pub(super) async fn prepare_last_part_regeneration(
    service: &OrchestratorContext,
    project_id: i64,
) -> Result<PreparedPartRegeneration, AppError> {
    let context =
        load_part_generation_context(service.database.connection(), project_id, None, None).await?;
    let target = context
        .parts
        .last()
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("没有分部可重新生成".to_string()))?;
    prepare_part_regeneration(project_id, target, context, true)
}

pub(super) async fn prepare_part_regeneration_by_id(
    service: &OrchestratorContext,
    project_id: i64,
    part_id: i64,
) -> Result<PreparedPartRegeneration, AppError> {
    let (target, context) = futures::join!(
        service.get_part(part_id),
        load_part_generation_context(service.database.connection(), project_id, None, None),
    );
    let target = target?;
    if target.project_id != project_id {
        return Err(AppError::InvalidArgument(format!(
            "分部大纲 {part_id} 不属于项目 {project_id}"
        )));
    }
    let context = context?;
    prepare_part_regeneration(project_id, target, context, false)
}

fn prepare_part_regeneration(
    project_id: i64,
    target: PartOutlineRecord,
    context: PartOutlineGenerationContext,
    rebind_chapter_outlines: bool,
) -> Result<PreparedPartRegeneration, AppError> {
    validate_part_outline_request(target.part_number, target.start_chapter, target.end_chapter)?;
    ensure_no_part_outline_conflict(
        &context.parts,
        target.part_number,
        target.start_chapter,
        target.end_chapter,
        Some(target.id),
    )?;
    let previous_parts = context
        .parts
        .iter()
        .filter(|part| part.part_number < target.part_number)
        .cloned()
        .collect();
    Ok(PreparedPartRegeneration {
        project_id,
        target,
        context,
        previous_parts,
        rebind_chapter_outlines,
    })
}
