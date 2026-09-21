//! Provider request snapshot construction owned by the video domain.

use serde_json::{Value, json};

pub fn provider_request_snapshot_value(
    key: &str,
    title: &str,
    kind: &str,
    prompt_name: Option<&str>,
    note: Option<String>,
    request: Value,
) -> Value {
    json!({
        "schema_version": 1,
        "captured_at": chrono::Utc::now().to_rfc3339(),
        "key": key,
        "title": title,
        "kind": kind,
        "prompt_name": prompt_name,
        "state": "ready",
        "note": note,
        "request": request,
    })
}

#[cfg(test)]
mod tests {
    use super::provider_request_snapshot_value;
    use serde_json::json;

    #[test]
    fn provider_request_snapshot_keeps_contract_fields() {
        let snapshot = provider_request_snapshot_value(
            "chapter_video",
            "章节视频 provider 请求",
            "video_provider_request",
            Some("chapter_video_prompt"),
            Some("生成时请求快照".to_string()),
            json!({"prompt": "video"}),
        );

        assert_eq!(snapshot["schema_version"], 1);
        assert!(
            snapshot["captured_at"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(snapshot["key"], "chapter_video");
        assert_eq!(snapshot["request"]["prompt"], "video");
    }
}
