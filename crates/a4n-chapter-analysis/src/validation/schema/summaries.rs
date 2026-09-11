use serde_json::{Map, Value};

use super::super::fields::{
    reject_unknown_fields, required_array, required_object, required_string,
};

pub(super) fn validate_summaries_schema(
    root: &Map<String, Value>,
) -> Result<(), a4n_error::AppError> {
    let summaries = required_object(root, "summaries", "chapter_analysis")?;
    reject_unknown_fields(
        summaries,
        &["compressed", "one_line", "keywords"],
        "chapter_analysis.summaries",
    )?;
    required_string(summaries, "compressed", "chapter_analysis.summaries")?;
    required_string(summaries, "one_line", "chapter_analysis.summaries")?;
    required_array(summaries, "keywords", "chapter_analysis.summaries")?;
    Ok(())
}
