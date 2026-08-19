use a4n_core::error::AppError;
use serde_json::{json, Value};

use a4n_llm::trace::context_aware_chat_request_trace_json;

pub fn chapter_generation_call_fingerprint_json(
    project_id: i64,
    chapter_number: i32,
    source_fingerprint_json: &Value,
    request: &a4n_llm::provider::ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "chapter_generation",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "source_fingerprint": source_fingerprint_json,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

pub fn validate_chapter_generation_source_fingerprint(
    expected: &Value,
    actual: &Value,
) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }
    Err("章节生成期间正式资料已变化，请按当前资料重新生成正文".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        chapter_generation_call_fingerprint_json, validate_chapter_generation_source_fingerprint,
    };
    use serde_json::json;

    #[test]
    fn rejects_changed_chapter_generation_sources() {
        let expected = json!({"outline": "旧章纲"});
        let actual = json!({"outline": "新章纲"});

        assert!(validate_chapter_generation_source_fingerprint(&expected, &expected).is_ok());
        assert!(
            validate_chapter_generation_source_fingerprint(&expected, &actual)
                .expect_err("changed sources must fail")
                .contains("正式资料已变化")
        );
    }

    #[test]
    fn call_fingerprint_records_formal_source_fingerprint() {
        let source = json!({"version": 1, "outline": "章纲"});
        let request =
            a4n_llm::provider::ChatRequest::system_user("system", "user", None, None, None);

        let fingerprint = chapter_generation_call_fingerprint_json(7, 3, &source, &request)
            .expect("fingerprint should serialize");

        assert_eq!(fingerprint["source_fingerprint"], source);
    }
}
