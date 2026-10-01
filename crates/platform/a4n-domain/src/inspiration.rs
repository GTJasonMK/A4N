//! Pure inputs for inspiration-domain records.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::project::{required_text, validate_blueprint_chapter_count};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspirationMessageRecord {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
}

pub fn inspiration_messages_llm_context(messages: &[InspirationMessageRecord]) -> Vec<Value> {
    messages
        .iter()
        .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
        .map(|message| {
            serde_json::json!({
                "role": message.role,
                "content": message.content,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspirationResponse {
    pub ai_message: String,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    #[schemars(required, extend("type" = ["string", "null"]))]
    pub next_question: Option<String>,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    #[schemars(required, extend("type" = ["string", "null"]))]
    pub progress_summary: Option<String>,
    pub ui_control: InspirationUiControl,
    pub conversation_state: Map<String, Value>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspirationUiControl {
    pub r#type: InspirationControlKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<InspirationOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InspirationControlKind {
    SingleChoice,
    TextInput,
    InfoDisplay,
    InspiredOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspirationOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub key_elements: Vec<String>,
}

pub fn parse_inspiration_response(raw: &str) -> Result<InspirationResponse, String> {
    let object: Map<String, Value> = serde_json::from_str(raw)
        .map_err(|error| format!("灵感回复必须是严格 JSON 对象: {error}"))?;
    let response: InspirationResponse = serde_json::from_value(Value::Object(object))
        .map_err(|error| format!("灵感回复合同无效: {error}"))?;
    required_text(&response.ai_message, "ai_message")?;
    if response.is_complete {
        let chapter_count = response
            .conversation_state
            .get("chapter_count")
            .and_then(Value::as_f64)
            .filter(|count| count.fract() == 0.0)
            .ok_or("完成态 conversation_state.chapter_count 必须是整数")?;
        validate_blueprint_chapter_count(
            chapter_count as i32,
            "完成态 conversation_state.chapter_count",
        )?;
        let genre = response
            .conversation_state
            .get("genre")
            .and_then(Value::as_str)
            .ok_or("完成态 conversation_state.genre 必须是非空字符串")?;
        required_text(genre, "完成态 conversation_state.genre")?;
    }
    if response.ui_control.r#type == InspirationControlKind::InspiredOptions
        && response.ui_control.options.is_empty()
    {
        return Err("ui_control.options 不能为空".into());
    }
    for (index, option) in response.ui_control.options.iter().enumerate() {
        for (name, value) in [
            ("id", &option.id),
            ("label", &option.label),
            ("description", &option.description),
        ] {
            required_text(value, &format!("ui_control.options[{index}].{name}"))?;
        }
        if !(2..=3).contains(&option.key_elements.len()) {
            return Err(format!(
                "ui_control.options[{index}].key_elements 必须包含 2-3 个元素"
            ));
        }
        for element in &option.key_elements {
            required_text(
                element,
                &format!("ui_control.options[{index}].key_elements"),
            )?;
        }
    }
    Ok(response)
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspirationExchangeInsertInput {
    pub project_id: i64,
    pub user_input: String,
    pub assistant_output: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_complete_and_incomplete_replies_before_they_can_enter_history() {
        let response = json!({
            "ai_message": "请描述主角。",
            "next_question": null,
            "progress_summary": null,
            "ui_control": {"type": "info_display"},
            "conversation_state": {},
            "is_complete": false,
        });
        parse_inspiration_response(&response.to_string()).unwrap();
        for field in [
            "ai_message",
            "next_question",
            "progress_summary",
            "ui_control",
            "conversation_state",
            "is_complete",
        ] {
            let mut missing = response.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                parse_inspiration_response(&missing.to_string()).is_err(),
                "{field}"
            );
        }
        let mut complete = response.clone();
        complete["is_complete"] = json!(true);
        complete["conversation_state"] = json!({"chapter_count": 6});
        assert!(
            parse_inspiration_response(&complete.to_string())
                .unwrap_err()
                .contains("conversation_state.genre")
        );
        complete["conversation_state"]["genre"] = json!("都市悬疑");
        parse_inspiration_response(&complete.to_string()).unwrap();
        for count in [4.0, 10001.0, 6.5] {
            complete["conversation_state"]["chapter_count"] = json!(count);
            assert!(parse_inspiration_response(&complete.to_string()).is_err());
        }
        let mut options = response;
        options["ui_control"] = json!({"type": "inspired_options", "options": []});
        assert!(parse_inspiration_response(&options.to_string()).is_err());
        options["ui_control"]["options"] = json!([{
            "id": "direction-1", "label": "时间谜题", "description": "追踪钟楼的秘密",
            "key_elements": ["旧车站", "未来来信"]
        }]);
        parse_inspiration_response(&options.to_string()).unwrap();
        options["ui_control"]["options"][0]
            .as_object_mut()
            .unwrap()
            .remove("id");
        assert!(parse_inspiration_response(&options.to_string()).is_err());
        assert!(
            parse_inspiration_response(r#"["消息",null,null,{"type":"info_display"},{},false]"#)
                .is_err()
        );
    }
}
