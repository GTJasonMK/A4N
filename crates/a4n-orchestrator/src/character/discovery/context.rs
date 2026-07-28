use a4n_core::{chapter_analysis::summary_text, models::chapter::chapter};
use serde_json::{Map, Value, json};

pub fn character_llm_chapter_context(chapter: &chapter::Model) -> Value {
    let mut context = Map::new();
    context.insert(
        "chapter_number".to_string(),
        Value::from(chapter.chapter_number),
    );
    context.insert("title".to_string(), json!(chapter.title));

    if let Some(summary) = chapter_summary_for_character_prompt(chapter) {
        context.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(analysis) = chapter
        .analysis_data
        .as_ref()
        .and_then(compact_character_prompt_analysis)
    {
        context.insert("analysis".to_string(), analysis);
    }
    context.insert(
        "content".to_string(),
        Value::String(chapter.content.clone()),
    );
    Value::Object(context)
}

fn chapter_summary_for_character_prompt(chapter: &chapter::Model) -> Option<String> {
    chapter
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| chapter.analysis_data.as_ref().and_then(summary_text))
}

fn compact_character_prompt_analysis(analysis: &Value) -> Option<Value> {
    let mut compact = Map::new();
    copy_analysis_field(&mut compact, analysis, "metadata");
    copy_analysis_field(&mut compact, analysis, "character_states");
    copy_analysis_field(&mut compact, analysis, "key_events");
    copy_analysis_field(&mut compact, analysis, "foreshadowing");
    (!compact.is_empty()).then_some(Value::Object(compact))
}

fn copy_analysis_field(compact: &mut Map<String, Value>, analysis: &Value, key: &str) {
    if let Some(value) = analysis.get(key)
        && !value.is_null()
    {
        compact.insert(key.to_string(), value.clone());
    }
}
