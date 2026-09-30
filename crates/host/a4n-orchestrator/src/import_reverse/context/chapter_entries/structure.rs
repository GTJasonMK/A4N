use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::PartOutlineRecord;
use serde_json::{Map, Value, json};

use super::formatting::{add_optional_chapter_title, add_summary_or_preview};

pub(in crate::import_reverse::context) fn chapter_outline_entry(ch: &chapter::Model) -> Value {
    let mut entry = Map::new();
    entry.insert("chapter_number".to_string(), json!(ch.chapter_number));
    add_optional_chapter_title(&mut entry, ch);
    add_summary_or_preview(&mut entry, ch, 240);
    Value::Object(entry)
}

pub(in crate::import_reverse::context) fn chapter_part_entry(part: &PartOutlineRecord) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
        "theme": part.theme,
    })
}
