use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_string_array,
};

pub(super) fn validate_summaries_fields(value: Option<&Value>) -> Result<(), AppError> {
    let summaries = value.and_then(Value::as_object).ok_or_else(|| {
        analysis_materialize_error("chapter_summary_synthesis.summaries 必须是对象")
    })?;
    reject_materialize_unknown_fields(
        summaries,
        &["compressed", "one_line", "keywords"],
        "chapter_summary_synthesis.summaries",
    )?;
    required_materialize_string(
        summaries,
        "compressed",
        "chapter_summary_synthesis.summaries.compressed",
    )?;
    required_materialize_string(
        summaries,
        "one_line",
        "chapter_summary_synthesis.summaries.one_line",
    )?;
    validate_string_array(
        summaries.get("keywords"),
        "chapter_summary_synthesis.summaries.keywords",
    )
}
