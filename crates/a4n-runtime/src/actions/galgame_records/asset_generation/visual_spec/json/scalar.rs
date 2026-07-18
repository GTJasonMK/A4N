use serde_json::Value;

fn json_str<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

pub(in crate::actions::galgame_records::asset_generation) fn json_owned_string(
    value: &Value,
    field: &str,
) -> Option<String> {
    json_str(value, field).map(str::to_string)
}

pub(in crate::actions::galgame_records::asset_generation) fn required_visual_json_object<'a>(
    value: &'a Value,
    field: &str,
    spec_id: i64,
) -> Result<&'a Value, String> {
    match value.get(field) {
        Some(raw) if raw.is_object() => Ok(raw),
        Some(_) => Err(format!(
            "Galgame 视觉生成意图参数 {field} 必须是对象: spec_id={spec_id}"
        )),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"
        )),
    }
}

pub(in crate::actions::galgame_records::asset_generation) fn required_visual_json_str<'a>(
    value: &'a Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<&'a str, String> {
    match value.get(field) {
        Some(raw) => raw
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                format!("Galgame 视觉生成意图参数 {path} 必须是非空字符串: spec_id={spec_id}")
            }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}

pub(super) fn required_visual_json_i64(
    value: &Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    match value.get(field) {
        Some(raw) => raw.as_i64().ok_or_else(|| {
            format!("Galgame 视觉生成意图参数 {path} 必须是整数: spec_id={spec_id}")
        }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}
