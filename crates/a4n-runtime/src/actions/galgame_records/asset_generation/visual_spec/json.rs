mod arrays;
mod rules;
mod scalar;

pub(in crate::actions::galgame_records::asset_generation) use arrays::{
    required_visual_json_i64_array, required_visual_json_string_array,
};
pub(in crate::actions::galgame_records::asset_generation) use rules::{
    validate_galgame_visual_canonical_key, validate_optional_galgame_cg_importance,
    validate_visual_json_i64_equals,
};
pub(in crate::actions::galgame_records::asset_generation) use scalar::{
    json_owned_string, required_visual_json_object, required_visual_json_str,
};
