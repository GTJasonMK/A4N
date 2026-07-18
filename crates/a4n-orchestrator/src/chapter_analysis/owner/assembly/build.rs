mod metadata;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use metadata::assemble_metadata;

pub fn assemble_staged_chapter_analysis(
    facts: Value,
    summary: Value,
    character_states: Value,
    foreshadowing_events: Value,
) -> Result<Value, AppError> {
    let metadata = assemble_metadata(&facts, &summary)?;

    Ok(json!({
        "metadata": metadata,
        "summaries": summary.get("summaries").cloned().unwrap_or(Value::Null),
        "character_states": character_states.get("character_states").cloned().unwrap_or(Value::Null),
        "foreshadowing": foreshadowing_events.get("foreshadowing").cloned().unwrap_or(Value::Null),
        "key_events": foreshadowing_events.get("key_events").cloned().unwrap_or(Value::Null),
    }))
}
