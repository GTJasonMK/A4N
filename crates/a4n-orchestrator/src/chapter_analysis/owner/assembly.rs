mod build;
pub use build::assemble_staged_chapter_analysis;

use a4n_core::error::AppError;
use serde_json::Value;

fn required_stage_field(value: &Value, stage: &str, field: &str) -> Result<Value, AppError> {
    value
        .get(field)
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument(format!("{stage} 缺少必需字段 {field}")))
}
