use std::collections::HashMap;

use serde_json::{Value, json};

use super::word_count::ChapterGenerationWordCountRange;

pub fn upsert_chapter_generation_variables(
    variables: &mut HashMap<String, Value>,
    chapter_number: i32,
    chapter_title: &str,
    outline_text: &str,
    writing_instruction: Option<&str>,
    reference_context: Option<&str>,
    rag_context: Option<&str>,
    word_count_range: ChapterGenerationWordCountRange,
) {
    variables.insert("chapter_number".to_string(), json!(chapter_number));
    variables.insert("chapter_title".to_string(), json!(chapter_title));
    variables.insert("chapter_outline".to_string(), json!(outline_text));
    variables.insert(
        "chapter_word_count_min".to_string(),
        json!(word_count_range.min),
    );
    variables.insert(
        "chapter_word_count_max".to_string(),
        json!(word_count_range.max),
    );
    variables.insert(
        "chapter_word_count_range".to_string(),
        json!(word_count_range.label()),
    );
    upsert_optional_text_variable(variables, "writing_instruction", writing_instruction);
    upsert_optional_text_variable(variables, "generation_reference_context", reference_context);
    upsert_optional_text_variable(variables, "generation_rag_context", rag_context);
}

fn upsert_optional_text_variable(
    variables: &mut HashMap<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    match value.and_then(non_empty_trimmed) {
        Some(value) => {
            variables.insert(key.to_string(), json!(value));
        }
        None => {
            variables.remove(key);
        }
    }
}

pub fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}
