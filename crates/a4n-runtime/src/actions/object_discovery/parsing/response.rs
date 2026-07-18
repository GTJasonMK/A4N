use super::objects::normalize_object;
use crate::actions::{
    json_contract::reject_unknown_fields, object_discovery::types::ParsedObjectProfileDraft,
};
use a4n_core::models::{chapter::chapter, project::text_anchor};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn object_discovery_response_json(content: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("物体档案发现结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    Ok(value)
}

pub(super) fn parse_object_discovery_output_value(
    value: &Value,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<ParsedObjectProfileDraft>, String> {
    if !value.is_object() {
        return Err("物体档案发现结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(value, "root", &["objects"])?;
    let objects = value
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| "物体档案发现字段 objects 必须是数组".to_string())?;
    let mut seen = BTreeSet::new();
    let mut drafts = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        let draft = normalize_object(index, object, chapters, text_anchors)?;
        let key = draft.draft.name.trim().to_lowercase();
        if !seen.insert(key) {
            return Err(format!("objects[{index}] 与前序物体重复"));
        }
        drafts.push(draft);
    }
    Ok(drafts)
}
