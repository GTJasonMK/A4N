//! 已优化的完整 provider 提交文本（metadata 读写）。

use a4n_core::video_execution::OPTIMIZED_PROVIDER_PROMPT_METADATA_KEY;
use serde_json::{Map, Value};

/// 读取已优化的完整 provider 文本（trim 后非空）。
pub fn optimized_provider_prompt_text(metadata: Option<&Value>) -> Option<String> {
    metadata
        .and_then(|value| value.get(OPTIMIZED_PROVIDER_PROMPT_METADATA_KEY))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// 写入已优化全文；`None` 表示清除该字段。
pub fn with_optimized_provider_prompt_metadata(
    metadata: Option<Value>,
    optimized_provider_prompt: Option<&str>,
) -> Value {
    let mut root = match metadata {
        Some(Value::Object(map)) => map,
        Some(_) | None => Map::new(),
    };
    match optimized_provider_prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(text) => {
            root.insert(
                OPTIMIZED_PROVIDER_PROMPT_METADATA_KEY.to_string(),
                Value::String(text.to_string()),
            );
        }
        None => {
            root.remove(OPTIMIZED_PROVIDER_PROMPT_METADATA_KEY);
        }
    }
    Value::Object(root)
}

/// 手动编辑 visual/motion 后应丢弃已优化全文，避免继续提交过期文本。
pub fn clear_optimized_provider_prompt_metadata(metadata: Option<Value>) -> Option<Value> {
    let Some(Value::Object(mut root)) = metadata else {
        return metadata;
    };
    root.remove(OPTIMIZED_PROVIDER_PROMPT_METADATA_KEY);
    Some(Value::Object(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_clears_optimized_provider_prompt() {
        let meta = with_optimized_provider_prompt_metadata(None, Some("  full prompt  "));
        assert_eq!(
            optimized_provider_prompt_text(Some(&meta)).as_deref(),
            Some("full prompt")
        );
        let cleared = clear_optimized_provider_prompt_metadata(Some(meta)).unwrap();
        assert_eq!(optimized_provider_prompt_text(Some(&cleared)), None);
    }
}
