use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::source_text_hash;
use chrono::Utc;
use serde_json::{Value, json};

pub fn analysis_meta(ch: &chapter::Model) -> Value {
    json!({
        "project_id": ch.project_id,
        "chapter_id": ch.id,
        "chapter_number": ch.chapter_number,
        "content_sha256": source_text_hash(&ch.content),
        "generated_at": Utc::now().naive_utc().to_string(),
    })
}
