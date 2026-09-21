mod entity_states;
mod occurrences;
mod profiles;
mod transitions;
mod validation;

use a4n_domain::json_contract::reject_unknown_fields;
use serde_json::Value;

pub(super) use entity_states::parse_scene_entity_states;
pub(super) use occurrences::parse_scene_occurrences;
pub(super) use profiles::{ParsedSceneProfiles, parse_scene_profiles};
pub(super) use transitions::parse_scene_transitions;

pub(super) fn parse_scene_stage_response<T>(
    content: &str,
    prompt_name: &str,
    root_field: &str,
    parse: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<(Vec<Value>, T), String> {
    let mut value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("{prompt_name} 结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err(format!("{prompt_name} 结果必须是 JSON 对象"));
    }
    reject_unknown_fields(&value, prompt_name, &[root_field])?;
    let parsed = parse(&value)?;
    match value[root_field].take() {
        Value::Array(output) => Ok((output, parsed)),
        _ => Err(format!("{prompt_name}.{root_field} 必须是数组")),
    }
}
