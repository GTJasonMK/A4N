mod fields;
mod schema;

use crate::error::AppError;
use serde_json::{Map, Value};

pub fn validate_saved_chapter_analysis(value: &Value) -> Result<(), AppError> {
    let mut root = value
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("chapter_analysis 必须是 JSON 对象".into()))?;
    root.remove("_meta");
    validate_chapter_analysis_schema(&root)
}

pub fn validate_chapter_analysis_schema(root: &Map<String, Value>) -> Result<(), AppError> {
    schema::validate_analysis_schema(root)
}

fn analysis_schema_error(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests;
