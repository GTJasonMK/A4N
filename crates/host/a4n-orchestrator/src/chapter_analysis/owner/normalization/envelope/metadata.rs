use a4n_db_models::models::chapter::chapter;
use a4n_domain::source_text_hash;
use chrono::Utc;
use serde_json::{Value, json};

pub fn analysis_meta(ch: &chapter::Model) -> Value {
    json!({
        "project_id": ch.project_id,
        "chapter_id": ch.id,
        "chapter_number": ch.chapter_number,
        "content_sha256": source_text_hash(&ch.content),
        "fact_feedback_schema_version": a4n_domain::fact_feedback::FACT_FEEDBACK_SCHEMA_VERSION,
        "generated_at": Utc::now().naive_utc().to_string(),
    })
}
