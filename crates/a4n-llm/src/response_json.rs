use a4n_error::AppError;
use serde_json::Value;

/// Parses an LLM response as a strict JSON object without accepting fences or
/// other non-JSON wrappers.
pub fn parse_llm_json_object(content: &str, context: &str) -> Result<Value, AppError> {
    a4n_domain::json_contract::parse_llm_json_object(content, context)
        .map_err(AppError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use a4n_error::AppError;
    use serde_json::json;

    use super::parse_llm_json_object;

    #[test]
    fn accepts_object() {
        let value = parse_llm_json_object(r#"{"chapter_number":1,"summary":"ok"}"#, "导入分析")
            .expect("valid json object");

        assert_eq!(value["chapter_number"], json!(1));
    }

    #[test]
    fn rejects_plain_text() {
        let err = parse_llm_json_object("这是一段分析文本", "导入分析")
            .expect_err("plain text should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_array() {
        let err = parse_llm_json_object(r#"[{"chapter_number":1}]"#, "reverse_outline")
            .expect_err("array should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_markdown_fence() {
        let err = parse_llm_json_object("```json\n{\"title\":\"x\"}\n```", "reverse_blueprint")
            .expect_err("markdown fence should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }
}
