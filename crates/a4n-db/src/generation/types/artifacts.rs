use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::GenerationDbError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunTraceTarget {
    pub target_kind: String,
    pub target_table: String,
    pub target_id: i64,
}

pub fn append_generation_artifact_target(
    artifact_refs: Option<Value>,
    target: GenerationRunTraceTarget,
) -> Result<Value, GenerationDbError> {
    let target = serde_json::json!({
        "target_kind": target.target_kind,
        "target_table": target.target_table,
        "target_id": target.target_id,
    });
    match artifact_refs {
        Some(Value::Object(mut map)) => {
            let targets = match map.remove("targets") {
                Some(Value::Array(mut targets)) => {
                    targets.push(target);
                    Value::Array(targets)
                }
                Some(_) => {
                    return Err(GenerationDbError::InvalidArgument(
                        "generation artifact_refs_json.targets 必须是数组".to_string(),
                    ));
                }
                None => Value::Array(vec![target]),
            };
            map.insert("targets".to_string(), targets);
            Ok(Value::Object(map))
        }
        Some(_) => Err(GenerationDbError::InvalidArgument(
            "generation artifact_refs_json 必须是对象".to_string(),
        )),
        None => Ok(serde_json::json!({ "targets": [target] })),
    }
}
