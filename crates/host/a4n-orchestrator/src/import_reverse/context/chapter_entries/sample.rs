use a4n_db_models::models::chapter::chapter;
use serde_json::{Map, Value, json};

use super::analysis::chapter_analysis_context;
use super::formatting::{add_optional_chapter_title, text_preview};

pub(in crate::import_reverse::context) fn sample_chapter_entry(
    ch: &chapter::Model,
) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("chapter_number".to_string(), json!(ch.chapter_number));
    add_optional_chapter_title(&mut entry, ch);
    entry.insert(
        "content".to_string(),
        json!(text_preview(&ch.content, 6000)),
    );
    if let Some(analysis) = chapter_analysis_context(ch)? {
        entry.insert("analysis".to_string(), analysis);
    }
    Ok(Value::Object(entry))
}
