use super::*;

mod portraits;
mod prompt_artifacts;
mod records;

pub(crate) use portraits::galgame_portrait_spec_references_lines;
#[cfg(test)]
pub(in crate::galgame) use portraits::validate_galgame_portrait_spec_target_input;
pub use portraits::{
    completed_galgame_portrait_record, galgame_portrait_spec_target_profile,
    galgame_webgal_portraits, primary_galgame_portrait_reference_image_ids,
};
pub use prompt_artifacts::{
    galgame_prompt_artifact_by_generation_spec, mark_galgame_generation_spec_status_record,
    upsert_galgame_prompt_artifact_record,
};
#[cfg(test)]
pub(in crate::galgame) use records::validate_galgame_asset_generation_spec_query;
pub use records::{
    galgame_asset_generation_specs, galgame_generation_spec_content_changed,
    upsert_galgame_asset_plan_spec_records_transaction,
};
