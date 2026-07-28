pub mod fields;
pub mod parsing;
pub mod part_context;
pub mod persistence;
pub mod prompt;

#[cfg(test)]
pub use fields::optional_i32;
pub use fields::{required_array, required_i32, required_object, required_string};
pub use parsing::{
    chapter_outline_batch_response_schema, parse_chapter_outline_batch, parse_outline_json_object,
    reject_keys, reject_unknown_keys, validate_materialized_chapter_outline_batch,
};
pub use part_context::{
    ensure_focus_part_in_context, requested_part_for_chapter, required_part_summary,
    validate_chapter_generation_range, validate_requested_part_context,
};
#[cfg(test)]
pub use persistence::sync_chapter_outlines_by_number;
pub use persistence::{
    insert_chapter_outline_from_json, sync_chapter_outlines_by_number_transaction,
    update_chapter_outline_from_json,
};
pub use prompt::{
    beat_mapping_manifest_user_message, chapter_outline_generation_manifest_user_message,
    chapter_outline_generation_max_tokens, parse_beat_sheet_json,
    validate_chapter_outline_generation_request,
};
