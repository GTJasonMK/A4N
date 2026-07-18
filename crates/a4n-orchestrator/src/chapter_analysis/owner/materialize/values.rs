use a4n_core::error::AppError;

mod enums;
mod normalization;
mod required;
mod source_spans;
mod string_array;
mod unknown_fields;
pub(in crate::chapter_analysis::owner) use enums::validate_required_string_enum;
pub(super) use normalization::{normalize_optional_string, normalize_string_array};
pub(in crate::chapter_analysis::owner) use required::required_materialize_string;
pub(in crate::chapter_analysis::owner) use source_spans::validate_source_span_ids;
pub(in crate::chapter_analysis::owner) use string_array::validate_string_array;
pub(in crate::chapter_analysis::owner) use unknown_fields::reject_materialize_unknown_fields;

pub(in crate::chapter_analysis::owner) fn analysis_materialize_error(
    message: impl Into<String>,
) -> AppError {
    AppError::InvalidArgument(message.into())
}
