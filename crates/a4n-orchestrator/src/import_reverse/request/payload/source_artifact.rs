use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::validation::required_reverse_text;

pub(in crate::import_reverse::request) fn reverse_engineer_source_artifact(
    content_sample: &str,
) -> Result<Value, AppError> {
    let content_sample = required_reverse_text(content_sample, "content_sample")?;

    Ok(match serde_json::from_str::<Value>(content_sample) {
        Ok(value) => json!({ "source_context": value }),
        Err(_) => json!({ "source_text": content_sample }),
    })
}
