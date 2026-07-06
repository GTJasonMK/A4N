mod model;
mod parsing;
mod persistence;
mod plan;
mod prompt;
mod validation;

#[cfg(test)]
pub(super) use model::PartGenerationPlan;
pub(super) use model::PartOutlineDraft;
pub(super) use parsing::parse_part_outline_single_response;
#[cfg(test)]
pub(super) use persistence::chapter_outline_ids_in_part_range;
pub(super) use persistence::{
    chapter_outline_ids_in_part_ranges, delete_chapter_outlines_in_part_range,
    part_outline_active_model, part_outline_model_from_json, sync_chapter_outline_part_assignments,
    sync_part_outlines, update_part_outline_active_model,
};
#[cfg(test)]
pub(super) use plan::validate_part_generation_inputs;
pub(super) use plan::{
    resolve_part_generation_plan, validate_existing_parts_for_latest_delete,
    validate_existing_parts_for_plan,
};
pub(super) use prompt::{PART_GENERATION_OUTPUT_CONTRACT, part_generation_manifest_user_message};
pub(super) use validation::{
    ensure_continue_part_request, ensure_no_part_outline_conflict, validate_part_outline_request,
};
