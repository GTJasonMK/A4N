//! Old regeneration outputs remain audit evidence, never a current executable plan.
use serde_json::{Value, json};

const ARCHIVE_VERSION: &str = "legacy_galgame_stage_outputs_v1";

pub fn archived_stage_outputs(value: &Value) -> Result<bool, String> {
    if value.get("schema_version").and_then(Value::as_str) != Some(ARCHIVE_VERSION) {
        return Ok(false);
    }
    let object = value.as_object().ok_or("历史同步回执必须是对象")?;
    if object.len() != 3 || !object.get("original").is_some_and(Value::is_object) {
        return Err("历史同步回执字段无效".into());
    }
    let expected = crate::generation::canonical_json_sha256(&object["original"])
        .map_err(|error| error.to_string())?;
    if object.get("sha256").and_then(Value::as_str) != Some(expected.as_str()) {
        return Err("历史同步回执内容哈希不一致".into());
    }
    Ok(true)
}

pub fn archive_legacy_source_sync_payload(payload: &mut Value) -> Result<bool, String> {
    let Some(stages) = payload.pointer_mut("/result/stage_outputs") else {
        return Ok(false);
    };
    if archived_stage_outputs(stages)? || stages.get("shooting_script_projection").is_none() {
        return Ok(false);
    }
    let keys = [
        "shooting_script_projection",
        "script_adaptation",
        "character_staging",
        "visual_planning",
        "branch_design",
    ];
    let object = stages.as_object().ok_or("旧同步阶段输出必须是对象")?;
    if object.len() != keys.len()
        || keys
            .iter()
            .any(|key| !object.get(*key).is_some_and(Value::is_object))
    {
        return Err("旧同步阶段回执不完整，不能转换为已归档记录".into());
    }
    let sha256 =
        crate::generation::canonical_json_sha256(stages).map_err(|error| error.to_string())?;
    *stages = json!({"schema_version":ARCHIVE_VERSION,"original":stages.clone(),"sha256":sha256});
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_sync_keeps_original_ids_and_outputs_with_verified_hash() {
        let original = json!({"shooting_script_projection":{"scenes":[{"shooting_script_id":7}]},
            "script_adaptation":{},"character_staging":{},"visual_planning":{},"branch_design":{}});
        let mut payload = json!({"result":{"stage_outputs":original}});
        assert!(archive_legacy_source_sync_payload(&mut payload).unwrap());
        assert_eq!(payload["result"]["stage_outputs"]["original"], original);
        assert!(archived_stage_outputs(&payload["result"]["stage_outputs"]).unwrap());
        assert!(!archive_legacy_source_sync_payload(&mut payload).unwrap());
        payload["result"]["stage_outputs"]["original"]["visual_planning"] = json!({"changed":true});
        assert!(archived_stage_outputs(&payload["result"]["stage_outputs"]).is_err());
    }
}
