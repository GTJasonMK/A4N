use super::super::OptimizedMangaPromptFields;
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::Value;

pub(in crate::actions::manga_records) fn manga_prompt_optimization_checkpoint_output(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(OptimizedMangaPromptFields, Option<Value>, Option<String>), String> {
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    let checkpoint_parsed: OptimizedMangaPromptFields =
        serde_json::from_value(parsed_output.clone())
            .map_err(|err| format!("{call_key}.parsed_output 字段不符合要求: {err}"))?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?,
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok((checkpoint_parsed, usage, finish_reason))
}
