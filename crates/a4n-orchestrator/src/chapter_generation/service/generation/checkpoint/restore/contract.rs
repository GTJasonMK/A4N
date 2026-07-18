use a4n_core::error::AppError;
use serde_json::Value;

use super::ChapterGenerationCheckpoint;

pub(super) fn parse_completed_chapter_generation_checkpoint(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<ChapterGenerationCheckpoint, AppError> {
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.response_content 必须是字符串"))
        })?;
    let parsed_content = checkpoint_json
        .get("parsed_output")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.parsed_output.content 必须是字符串"))
        })?;
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{call_key}.finish_reason 必须是字符串或 null"
                    ))
                })?
                .to_string(),
        ),
    };
    Ok(ChapterGenerationCheckpoint {
        response_content,
        parsed_content,
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_completed_chapter_generation_checkpoint;

    #[test]
    fn parses_completed_checkpoint_contract() {
        let checkpoint = parse_completed_chapter_generation_checkpoint(
            &json!({
                "response_content": "正文",
                "parsed_output": { "content": "规范正文" },
                "finish_reason": "stop"
            }),
            "chapter_generation:chapter:1",
        )
        .expect("valid checkpoint contract");

        assert_eq!(checkpoint.response_content, "正文");
        assert_eq!(checkpoint.parsed_content, "规范正文");
        assert_eq!(checkpoint.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn accepts_missing_finish_reason_as_none() {
        let checkpoint = parse_completed_chapter_generation_checkpoint(
            &json!({
                "response_content": "正文",
                "parsed_output": { "content": "正文" }
            }),
            "chapter_generation:chapter:2",
        )
        .expect("finish reason is optional");

        assert_eq!(checkpoint.finish_reason, None);
    }

    #[test]
    fn rejects_missing_parsed_content() {
        let error = match parse_completed_chapter_generation_checkpoint(
            &json!({ "response_content": "正文", "parsed_output": {} }),
            "chapter_generation:chapter:3",
        ) {
            Ok(_) => panic!("parsed content is required"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "参数无效：chapter_generation:chapter:3.parsed_output.content 必须是字符串"
        );
    }

    #[test]
    fn rejects_non_string_finish_reason() {
        let error = match parse_completed_chapter_generation_checkpoint(
            &json!({
                "response_content": "正文",
                "parsed_output": { "content": "正文" },
                "finish_reason": 1
            }),
            "chapter_generation:chapter:4",
        ) {
            Ok(_) => panic!("finish reason must be string or null"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "参数无效：chapter_generation:chapter:4.finish_reason 必须是字符串或 null"
        );
    }
}
