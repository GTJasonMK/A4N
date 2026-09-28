use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

pub struct ScriptReferenceChange {
    pub id: i64,
    pub revision: i32,
    pub old_sha256: String,
    pub sha256: String,
    pub old_source_revision_key: String,
    pub source_revision_key: String,
}

/// Only typed identity fields are inspected. Never pass raw model requests or
/// responses through this conversion; those remain the original audit record.
pub fn referenced_script_ids(value: &Value) -> BTreeSet<i64> {
    let mut ids = BTreeSet::new();
    collect(value, &mut ids);
    ids
}

fn collect(value: &Value, ids: &mut BTreeSet<i64>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("shooting_script_id").and_then(Value::as_i64) {
                ids.insert(id);
            }
            if let Some(id) = object
                .get("shooting_script_identity")
                .and_then(|value| value.get("record_id"))
                .and_then(Value::as_i64)
            {
                ids.insert(id);
            }
            for value in object.values() {
                collect(value, ids);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect(value, ids);
            }
        }
        _ => {}
    }
}

pub fn rewrite_film_references(
    value: &mut Value,
    scripts: &BTreeMap<i64, ScriptReferenceChange>,
    plans: &BTreeMap<i64, i64>,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if let Some(identity) = object
                .get_mut("shooting_script_identity")
                .filter(|value| !value.is_null())
            {
                let identity = identity.as_object_mut().ok_or("视频剧本身份必须是对象")?;
                rewrite_script(identity, "record_id", "revision", "script_sha256", scripts)?;
            }
            if object.contains_key("shooting_script_id") {
                rewrite_script(
                    object,
                    "shooting_script_id",
                    "shooting_script_revision",
                    "shooting_script_sha256",
                    scripts,
                )?;
            }
            if let Some(identity) = object
                .get_mut("visual_plan_source")
                .filter(|value| !value.is_null())
            {
                let old = identity
                    .get("plan_id")
                    .and_then(Value::as_i64)
                    .ok_or("剧本导演身份缺少 plan_id")?;
                identity["plan_id"] = plans
                    .get(&old)
                    .copied()
                    .ok_or_else(|| format!("视频缺少所属导演方案: {old}"))?
                    .into();
            }
            for field in ["visual_plan_id"] {
                if let Some(id) = object.get_mut(field).filter(|value| !value.is_null()) {
                    let old = id.as_i64().ok_or("导演引用必须是整数")?;
                    *id = plans
                        .get(&old)
                        .copied()
                        .ok_or_else(|| format!("视频缺少所属导演方案: {old}"))?
                        .into();
                }
            }
            for (key, nested) in object.iter_mut() {
                if key != "shooting_script_identity" {
                    rewrite_film_references(nested, scripts, plans)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                rewrite_film_references(value, scripts, plans)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn rewrite_script(
    object: &mut Map<String, Value>,
    id_field: &str,
    revision_field: &str,
    hash_field: &str,
    scripts: &BTreeMap<i64, ScriptReferenceChange>,
) -> Result<(), String> {
    if object.get(id_field).is_some_and(Value::is_null) {
        return Ok(());
    }
    let old = object
        .get(id_field)
        .and_then(Value::as_i64)
        .ok_or("视频剧本引用必须是整数")?;
    let mapping = scripts
        .get(&old)
        .ok_or_else(|| format!("视频缺少所属剧本: {old}"))?;
    object.insert(id_field.into(), mapping.id.into());
    if object.get(revision_field).and_then(Value::as_i64) == Some(i64::from(mapping.revision))
        && object.get(hash_field).and_then(Value::as_str) == Some(mapping.old_sha256.as_str())
    {
        object.insert(hash_field.into(), mapping.sha256.clone().into());
    }
    if object.get("source_revision_key").and_then(Value::as_str)
        == Some(mapping.old_source_revision_key.as_str())
    {
        object.insert(
            "source_revision_key".into(),
            mapping.source_revision_key.clone().into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn copies_execution_identity_without_advancing_stale_content() {
        let scripts = BTreeMap::from([(
            7,
            ScriptReferenceChange {
                id: 17,
                revision: 2,
                old_sha256: "current".into(),
                sha256: "mapped".into(),
                old_source_revision_key: "live".into(),
                source_revision_key: "snapshot".into(),
            },
        )]);
        let mut value = json!({"video_execution_identity": {"shooting_script_identity": {
            "record_id": 7, "revision": 1, "script_sha256": "older", "source_revision_key": "live"
        }}});
        assert_eq!(referenced_script_ids(&value), BTreeSet::from([7]));
        rewrite_film_references(&mut value, &scripts, &BTreeMap::new()).unwrap();
        let identity = &value["video_execution_identity"]["shooting_script_identity"];
        assert_eq!(identity["record_id"], 17);
        assert_eq!(identity["script_sha256"], "older");
        assert_eq!(identity["source_revision_key"], "snapshot");
    }
}
