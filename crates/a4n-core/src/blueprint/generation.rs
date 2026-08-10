use std::collections::HashMap;

use serde_json::Value;

use crate::error::AppError;

pub const BLUEPRINT_GENERATE_PROGRESS_TOTAL: u32 = 12;
pub const BLUEPRINT_STAGE_SYSTEM_PROMPT: &str = "blueprint_stage_system";
pub const BLUEPRINT_MIN_TOTAL_CHAPTERS: i32 = 5;
pub const BLUEPRINT_MAX_TOTAL_CHAPTERS: i32 = 10_000;
pub const PART_OUTLINE_THRESHOLD: i32 = 50;
pub const CHAPTERS_PER_PART: i32 = 25;

pub fn needs_part_outlines_for_chapters(total_chapters: i32) -> bool {
    total_chapters > PART_OUTLINE_THRESHOLD
}

pub fn validate_blueprint_chapter_count(value: i32, context: &str) -> Result<(), AppError> {
    if (BLUEPRINT_MIN_TOTAL_CHAPTERS..=BLUEPRINT_MAX_TOTAL_CHAPTERS).contains(&value) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{context} 必须在 {BLUEPRINT_MIN_TOTAL_CHAPTERS} 到 {BLUEPRINT_MAX_TOTAL_CHAPTERS} 之间"
        )))
    }
}

pub fn build_blueprint_generation_payload(
    novel_title: &str,
    genre: &str,
    target_chapters: i32,
    variables: &HashMap<String, Value>,
) -> Result<Value, AppError> {
    validate_blueprint_chapter_count(target_chapters, "requested_chapter_count")?;

    let mut payload = serde_json::Map::new();
    payload.insert(
        "novel_title".to_string(),
        Value::String(novel_title.to_string()),
    );
    payload.insert("genre".to_string(), Value::String(genre.to_string()));
    payload.insert(
        "requested_chapter_count".to_string(),
        Value::from(target_chapters),
    );
    for key in [
        "inspiration_transcript",
        "conversation_state",
        "progress_summary",
        "next_question",
        "project",
    ] {
        if let Some(value) = variables.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    Ok(Value::Object(payload))
}
