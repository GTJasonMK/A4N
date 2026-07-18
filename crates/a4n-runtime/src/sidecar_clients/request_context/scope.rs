use serde_json::Value;

pub(super) fn payload_project_scope(payload: &Value) -> Result<(Option<i64>, Option<i64>), String> {
    let Value::Object(map) = payload else {
        return Ok((None, None));
    };
    Ok((
        payload_i64_field(map, "project_id", &["project_id", "projectId"])?,
        payload_i64_field(
            map,
            "root_project_id",
            &["root_project_id", "rootProjectId"],
        )?,
    ))
}

fn payload_i64_field(
    map: &serde_json::Map<String, Value>,
    field: &str,
    keys: &[&str],
) -> Result<Option<i64>, String> {
    let mut parsed = None;
    for key in keys {
        let Some(value) = map.get(*key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let id = value_as_i64(value)
            .ok_or_else(|| format!("sidecar payload.{key} 必须是正整数，实际为 {value}"))?;
        if id <= 0 {
            return Err(format!("sidecar payload.{key} 必须是正整数，实际为 {id}"));
        }
        if let Some(previous) = parsed
            && previous != id
        {
            return Err(format!(
                "sidecar payload 的 {field} 别名字段不一致: {previous} != {id}"
            ));
        }
        parsed = Some(id);
    }
    Ok(parsed)
}

pub(super) fn merge_project_scope(
    field: &str,
    context_value: &mut Option<i64>,
    payload_value: Option<i64>,
) -> Result<(), String> {
    match (*context_value, payload_value) {
        (Some(context_value), Some(payload_value)) if context_value != payload_value => {
            Err(format!(
                "sidecar request context.{field} 与 payload.{field} 不一致: {context_value} != {payload_value}"
            ))
        }
        (None, Some(payload_value)) => {
            *context_value = Some(payload_value);
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(in crate::sidecar_clients) fn value_as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}
