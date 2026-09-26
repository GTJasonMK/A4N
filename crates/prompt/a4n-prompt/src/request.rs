use a4n_error::AppError;
use serde_json::Value;

const JSON_USER_MESSAGE_INPUT_MARKER: &str = "\n\n输入：\n";
const DEFAULT_ROOT_KEY_ORDER: &[&str] = &["task", "constraints", "output_contract", "input"];

pub fn build_json_task_user_message(
    task: &str,
    response_requirement: &str,
    payload: &Value,
) -> Result<String, AppError> {
    build_json_user_message(
        &format!("请根据以下 JSON 输入{}", sentence_body(task)?),
        response_requirement,
        payload,
    )
}

fn build_json_user_message(
    lead: &str,
    response_requirement: &str,
    payload: &Value,
) -> Result<String, AppError> {
    let lead = sentence(lead, "JSON user message 缺少任务说明")?;
    let response_requirement = sentence(response_requirement, "JSON user message 缺少返回要求")?;
    let payload = compact_json_with_root_key_order(payload, DEFAULT_ROOT_KEY_ORDER)?;
    Ok(format!(
        "{lead}{response_requirement}{JSON_USER_MESSAGE_INPUT_MARKER}{payload}"
    ))
}

pub fn compact_json_with_root_key_order(
    value: &Value,
    root_key_order: &[&str],
) -> Result<String, AppError> {
    let Some(object) = value.as_object() else {
        return serde_json::to_string(value).map_err(AppError::Serialization);
    };
    if object.is_empty() {
        return Ok("{}".to_string());
    }

    let mut keys = Vec::with_capacity(object.len());
    for key in root_key_order {
        if object.contains_key(*key) && !keys.contains(key) {
            keys.push(*key);
        }
    }
    for key in object.keys() {
        let key = key.as_str();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    let mut rendered = String::from("{");
    for (index, key) in keys.iter().enumerate() {
        let key_text = serde_json::to_string(key).map_err(AppError::Serialization)?;
        let value_text = serde_json::to_string(&object[*key]).map_err(AppError::Serialization)?;
        rendered.push_str(&key_text);
        rendered.push(':');
        rendered.push_str(&value_text);
        if index + 1 < keys.len() {
            rendered.push(',');
        }
    }
    rendered.push('}');
    Ok(rendered)
}

pub fn json_user_message_payload(content: &str) -> Option<&str> {
    content
        .split_once(JSON_USER_MESSAGE_INPUT_MARKER)
        .map(|(_, payload)| payload.trim())
        .filter(|payload| !payload.is_empty())
}

fn sentence_body(value: &str) -> Result<String, AppError> {
    let value = value.trim().trim_end_matches(['。', '.', '；', ';']).trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(
            "JSON user message 缺少任务说明".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn sentence(value: &str, error_message: &str) -> Result<String, AppError> {
    let value = value.trim().trim_end_matches(['。', '.', '；', ';']).trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(error_message.to_string()));
    }
    Ok(format!("{value}。"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_standard_json_task_user_message() {
        let message = build_json_task_user_message(
            "执行测试任务。",
            "只返回 output_contract 要求的严格 JSON 对象。",
            &json!({"input": 1}),
        )
        .expect("message");

        assert!(message.starts_with("请根据以下 JSON 输入执行测试任务。"));
        assert!(message.contains("只返回 output_contract 要求的严格 JSON 对象。"));
        assert_eq!(json_user_message_payload(&message), Some("{\"input\":1}"));
    }

    #[test]
    fn orders_stable_root_keys_before_dynamic_input() {
        let text = compact_json_with_root_key_order(
            &json!({
                "input": {"chapter": "dynamic"},
                "task": {"name": "step"},
                "output_contract": {"format": "json_object"},
                "z_extra": true
            }),
            DEFAULT_ROOT_KEY_ORDER,
        )
        .expect("ordered json");

        let task_index = text.find("\"task\"").expect("task");
        let contract_index = text.find("\"output_contract\"").expect("output_contract");
        let input_index = text.find("\"input\"").expect("input");
        let extra_index = text.find("\"z_extra\"").expect("extra");
        assert!(task_index < contract_index);
        assert!(contract_index < input_index);
        assert!(input_index < extra_index);
    }

    #[test]
    fn rejects_empty_message_parts() {
        let err = build_json_task_user_message(" ", "只返回 JSON", &json!({})).unwrap_err();
        assert!(err.to_string().contains("缺少任务说明"));

        let err = build_json_task_user_message("执行测试", " ", &json!({})).unwrap_err();
        assert!(err.to_string().contains("缺少返回要求"));
    }
}
