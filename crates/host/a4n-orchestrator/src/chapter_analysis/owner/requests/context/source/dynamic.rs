use serde_json::{Value, json};

pub fn chapter_analysis_dynamic_input(mut input: Value) -> Value {
    if let Some(object) = input.as_object_mut() {
        object.remove("content");
        object.remove("source_spans");
        object.insert(
            "chapter_source_context".to_string(),
            json!("章节正文和 source_spans 位于稳定上下文 chapter_analysis_source"),
        );
    }
    input
}
