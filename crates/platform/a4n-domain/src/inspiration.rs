//! Pure inputs for inspiration-domain records.

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspirationResponse {
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub ai_message: String,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    #[schemars(with = "crate::json_contract::RequiredNullable<String>")]
    pub next_question: Option<String>,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    #[schemars(with = "crate::json_contract::RequiredNullable<String>")]
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
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub id: String,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub label: String,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub description: String,
    #[schemars(length(min = 2, max = 3))]
    pub key_elements: Vec<String>,
}

pub fn inspiration_output_schema() -> Value {
    let mut schema = crate::json_contract::typed_output_schema::<InspirationResponse>();
    schema["$defs"]["InspirationOption"]["properties"]["key_elements"]["items"] =
        json!({"type":"string","minLength":1,"pattern":"\\S"});
    schema["allOf"] = json!([
        {"if":{"properties":{"is_complete":{"const":true}}},"then":{"properties":{"conversation_state":{"required":["chapter_count","genre"],"properties":{
            "chapter_count":{"type":"integer","minimum":crate::project::BLUEPRINT_MIN_GENERATION_CHAPTERS,"maximum":crate::project::BLUEPRINT_MAX_TOTAL_CHAPTERS},
            "genre":{"type":"string","minLength":1,"pattern":"\\S"}
        }}}}},
        {"if":{"properties":{"ui_control":{"properties":{"type":{"const":"inspired_options"}}}}},"then":{"properties":{"ui_control":{"required":["options"],"properties":{"options":{"minItems":1}}}}}}
    ]);
    schema
}

pub fn parse_inspiration_response(raw: &str) -> Result<InspirationResponse, String> {
    let value = crate::json_contract::parse_strict_json_value(raw)
        .map_err(|error| format!("灵感回复必须是严格 JSON 对象: {error}"))?;
    crate::json_contract::OutputSchemaValidator::new(&inspiration_output_schema())?
        .validate(&value)?;
    let response: InspirationResponse =
        serde_json::from_value(value).map_err(|error| format!("灵感回复合同无效: {error}"))?;
    Ok(response)
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspirationExchangeInsertInput {
    pub project_id: i64,
    pub user_input: String,
    pub assistant_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspirationMessageRecord {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: NaiveDateTime,
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
        let error = parse_inspiration_response(&complete.to_string()).unwrap_err();
        assert!(
            error.contains("conversation_state") && error.contains("genre"),
            "{error}"
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
