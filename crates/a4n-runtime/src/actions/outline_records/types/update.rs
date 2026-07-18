use serde::{Deserialize, Deserializer, de};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterOutlineInput {
    pub project_id: i64,
    pub outline_id: i64,
    #[serde(default, deserialize_with = "deserialize_optional_outline_title")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_summary")]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_beats")]
    pub beats: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_key_scenes")]
    pub key_scenes: Option<Value>,
}

fn deserialize_optional_outline_title<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "title")
}

fn deserialize_optional_outline_summary<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "summary")
}

fn deserialize_optional_outline_text<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(de::Error::custom(format!("章节大纲字段 {field} 不能为空")));
            }
            Ok(Some(trimmed.to_string()))
        }
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是字符串"
        ))),
    }
}

fn deserialize_optional_outline_beats<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "beats")
}

fn deserialize_optional_outline_key_scenes<'de, D>(
    deserializer: D,
) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "key_scenes")
}

fn deserialize_optional_outline_array<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(_) => Ok(Some(value)),
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是数组"
        ))),
    }
}
