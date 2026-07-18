use a4n_core::error::AppError;
use serde_json::Value;

mod optional;
mod scalars;
mod structure;
pub(in crate::blueprint::json) use optional::optional_string;
pub(in crate::blueprint::json) use scalars::{required_bool, required_i32, required_string};
pub(in crate::blueprint::json) use structure::{required_array, required_object_clone};

pub(in crate::blueprint::json) fn required_field<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Value, AppError> {
    json.get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必填字段 {key}")))
}
