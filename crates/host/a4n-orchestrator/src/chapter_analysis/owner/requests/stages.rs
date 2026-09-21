use a4n_error::AppError;
use serde_json::Value;

mod facts;
pub use facts::build_chapter_fact_extraction_request;
mod summary;
pub use summary::build_chapter_summary_synthesis_request;
mod character_state;
pub use character_state::build_chapter_character_state_request;
mod foreshadowing;
pub use foreshadowing::{
    build_chapter_foreshadowing_events_request, chapter_foreshadowing_resolution_context,
};

fn required_fact_field(facts: &Value, field: &str) -> Result<Value, AppError> {
    facts.get(field).cloned().ok_or_else(|| {
        AppError::InvalidArgument(format!("chapter_fact_extraction 缺少必需字段 {field}"))
    })
}
