pub mod model;
pub mod parsing;
pub mod persistence;
pub mod plan;
pub mod prompt;
pub mod validation;

#[cfg(test)]
pub use model::PartGenerationPlan;
pub use model::PartOutlineDraft;
pub use parsing::{
    parse_part_outline_single_response, part_outline_single_response_schema,
    validate_materialized_part_outline,
};
#[cfg(test)]
pub use persistence::{
    chapter_outline_ids_in_part_range, chapter_outline_ids_in_part_ranges,
    sync_chapter_outline_part_assignments, sync_part_outlines,
};
pub use persistence::{
    insert_part_outline_from_json, part_outline_model_from_json,
    sync_part_outlines_and_assign_chapters_transaction,
    update_part_outline_and_rebind_chapters_transaction, update_part_outline_from_json,
};
#[cfg(test)]
pub use plan::validate_part_generation_inputs;
pub use plan::{
    infer_chapters_per_part_from_existing, resolve_part_generation_plan,
    validate_existing_parts_for_plan,
};
pub use prompt::{PART_GENERATION_OUTPUT_CONTRACT, part_generation_manifest_user_message};
pub use validation::{
    ensure_continue_part_request, ensure_no_part_outline_conflict, validate_part_outline_request,
};
