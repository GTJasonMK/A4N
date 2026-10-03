use a4n_chapter_analysis::summary_text;
use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ChapterRecord;
use serde_json::{Map, Value, json};

pub fn character_llm_chapter_context(chapter: &ChapterRecord) -> Value {
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

pub fn character_llm_chapter_context_model(chapter: &chapter::Model) -> Value {
    character_llm_chapter_context(&ChapterRecord {
        id: chapter.id,
        project_id: chapter.project_id,
        outline_id: chapter.outline_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        content: chapter.content.clone(),
        word_count: chapter.word_count,
        status: chapter.status.clone(),
        summary: chapter.summary.clone(),
        style_tags: chapter.style_tags.clone(),
        analysis_data: chapter.analysis_data.clone(),
        created_at: chapter.created_at,
        updated_at: chapter.updated_at,
    })
}

fn chapter_summary_for_character_prompt(chapter: &ChapterRecord) -> Option<String> {
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
