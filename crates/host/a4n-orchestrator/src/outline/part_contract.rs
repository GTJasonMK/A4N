mod model;
mod parsing;
mod persistence;
mod plan;
mod prompt;
mod validation;

pub(super) use model::PartOutlineDraft;
pub(super) use parsing::parse_part_outline_single_response;
pub(super) use persistence::{
    insert_part_outline_from_json, part_outline_model_from_json,
    sync_part_outlines_and_assign_chapters_in, update_part_outline_and_rebind_chapters_in,
    update_part_outline_from_json,
};
#[cfg(test)]
pub(super) use plan::validate_part_generation_inputs;
pub(super) use plan::{
    infer_chapters_per_part_from_existing, resolve_part_generation_plan,
    validate_existing_parts_for_plan,
};
pub(super) use prompt::{PART_GENERATION_OUTPUT_CONTRACT, part_generation_manifest_user_message};
pub(super) use validation::{
    ensure_continue_part_request, ensure_no_part_outline_conflict, validate_part_outline_request,
};
