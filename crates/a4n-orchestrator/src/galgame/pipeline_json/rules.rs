use a4n_core::error::AppError;

mod indexes;
mod json;
mod text;

pub(super) use indexes::{
    ensure_contiguous_indexes, ensure_unique_indexes, ensure_unique_scene_indexes,
    require_nonempty, require_positive, require_positive_i64,
};
pub(super) use json::from_value;
pub(super) use text::{
    normalize_script_line_type, normalize_source_span_ids, normalize_text_vec, optional_text,
    required_text,
};

pub(super) fn invalid(message: String) -> AppError {
    AppError::InvalidArgument(message)
}
