use serde_json::Value;
use std::collections::BTreeMap;

pub const LEGACY_MEDIA_MODE: &str = "legacy_media_authoring";

pub fn uses_legacy_media_pipeline(kind: &str, mode: Option<&str>, input: Option<&Value>) -> bool {
    if mode == Some(LEGACY_MEDIA_MODE) {
        return false;
    }
    match kind {
        "manga_pipeline" => {
            input
                .and_then(|value| value.get("authoring_contract"))
                .and_then(Value::as_str)
                != Some(super::super::MEDIA_AUTHORING_PIPELINE_VERSION)
        }
        "galgame_pipeline" => mode != Some(super::super::MEDIA_AUTHORING_PIPELINE_VERSION),
        _ => false,
    }
}

/// Change diagnostic ownership only. Provider messages, source material and
/// original model responses remain the exact evidence of the saved call.
pub fn relocate_film_fingerprint(
    value: &mut Value,
    old_project: i64,
    project: i64,
    plans: &BTreeMap<i64, i64>,
) -> Result<(), String> {
    let object = value
        .as_object_mut()
        .ok_or("历史工作流输入指纹必须是对象")?;
    if object.get("project_id").and_then(Value::as_i64) == Some(old_project) {
        object.insert("project_id".into(), project.into());
    }
    if let Some(id) = object
        .get_mut("visual_plan_id")
        .filter(|value| !value.is_null())
    {
        let old = id.as_i64().ok_or("历史工作流导演身份无效")?;
        *id = plans
            .get(&old)
            .copied()
            .ok_or("历史工作流导演方案无法映射")?
            .into();
    }
    relocate_trace_owner(value, old_project, project);
    Ok(())
}

fn relocate_trace_owner(value: &mut Value, old: i64, new: i64) {
    match value {
        Value::Object(object) => {
            if object.get("messages").is_some_and(Value::is_array) {
                if let Some(trace) = object
                    .get_mut("trace_context")
                    .and_then(Value::as_object_mut)
                    && trace.get("project_id").and_then(Value::as_i64) == Some(old)
                {
                    trace.insert("project_id".into(), new.into());
                }
                return;
            }
            for (key, child) in object {
                if key != "messages" {
                    relocate_trace_owner(child, old, new);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                relocate_trace_owner(child, old, new);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn relocating_a_call_preserves_wire_messages_and_source_owners() {
        let mut input = json!({"project_id":1, "visual_plan_id":7, "source":{"project_id":1},
            "request":{"messages":[{"role":"user", "content":"project_id: 1"}], "trace_context":{"project_id":1}}});
        let messages = input["request"]["messages"].clone();
        relocate_film_fingerprint(&mut input, 1, 2, &BTreeMap::from([(7, 17)])).unwrap();
        assert_eq!(input["project_id"], 2);
        assert_eq!(input["visual_plan_id"], 17);
        assert_eq!(input["request"]["trace_context"]["project_id"], 2);
        assert_eq!(input["request"]["messages"], messages);
        assert_eq!(input["source"]["project_id"], 1);
    }
}
