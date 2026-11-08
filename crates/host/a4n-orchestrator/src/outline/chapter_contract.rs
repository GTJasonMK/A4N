mod fields;
mod parsing;
mod part_context;
mod persistence;
mod prompt;

#[cfg(test)]
pub(in crate::outline) use fields::optional_i32;
pub(in crate::outline) use fields::{
    required_array, required_i32, required_object, required_string,
};
pub(in crate::outline) use parsing::{parse_chapter_outline_batch, parse_outline_json_object};
#[cfg(test)]
pub(in crate::outline) use parsing::{reject_keys, reject_unknown_keys};
pub(in crate::outline) use part_context::{
    ensure_focus_part_in_context, requested_part_for_chapter, required_part_summary,
    validate_chapter_generation_range, validate_requested_part_context,
};
pub(in crate::outline) use persistence::outline_db_error;
pub(in crate::outline) use persistence::{
    insert_chapter_outline_from_json, sync_chapter_outlines_by_number_in,
    update_chapter_outline_from_json_in,
};
pub(in crate::outline) use prompt::{
    beat_mapping_manifest_user_message, chapter_outline_generation_manifest_user_message,
    parse_beat_sheet_json, validate_chapter_outline_generation_request,
};
