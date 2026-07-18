use serde_json::{Value, json};

pub(super) fn completed_chapter_generation_checkpoint_json(
    response_content: &str,
    parsed_content: &str,
    finish_reason: Option<&str>,
) -> Value {
    json!({
        "output": {"content": parsed_content},
        "parsed_output": {"content": parsed_content},
        "response_content": response_content,
        "usage": null,
        "finish_reason": finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::completed_chapter_generation_checkpoint_json;

    #[test]
    fn builds_completed_checkpoint_contract() {
        assert_eq!(
            completed_chapter_generation_checkpoint_json("原始正文", "规范正文", Some("stop")),
            json!({
                "output": {"content": "规范正文"},
                "parsed_output": {"content": "规范正文"},
                "response_content": "原始正文",
                "usage": null,
                "finish_reason": "stop",
            })
        );
    }

    #[test]
    fn serializes_missing_finish_reason_as_null() {
        let checkpoint = completed_chapter_generation_checkpoint_json("正文", "正文", None);

        assert_eq!(checkpoint["finish_reason"], Value::Null);
        assert_eq!(checkpoint["usage"], Value::Null);
    }
}
