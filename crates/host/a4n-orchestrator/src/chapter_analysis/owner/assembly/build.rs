mod metadata;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::required_stage_field;
use metadata::assemble_metadata;

pub fn assemble_staged_chapter_analysis(
    facts: Value,
    summary: Value,
    character_states: Value,
    foreshadowing_events: Value,
) -> Result<Value, AppError> {
    let metadata = assemble_metadata(&facts, &summary)?;
    let world_facts = required_stage_field(&facts, "chapter_fact_extraction", "world_facts")?;
    let summaries = required_stage_field(&summary, "chapter_summary_synthesis", "summaries")?;
    let character_states = required_stage_field(
        &character_states,
        "chapter_character_state",
        "character_states",
    )?;
    let foreshadowing = required_stage_field(
        &foreshadowing_events,
        "chapter_foreshadowing_events",
        "foreshadowing",
    )?;
    let key_events = required_stage_field(
        &foreshadowing_events,
        "chapter_foreshadowing_events",
        "key_events",
    )?;

    Ok(json!({
        "metadata": metadata,
        "summaries": summaries,
        "character_states": character_states,
        "foreshadowing": foreshadowing,
        "key_events": key_events,
        "world_facts": world_facts,
    }))
}
