use serde_json::Value;

use super::super::contracts::{RegenerateSceneStageOutputContext, SourceSyncReferenceSets};
use super::references::validate_stage_output_object;

pub(in super::super) fn validate_galgame_stage_outputs(
    value: &Value,
    refs: &SourceSyncReferenceSets,
    context: &RegenerateSceneStageOutputContext,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            validate_stage_output_object(object, refs, context, owner_kind, owner_id, path)?;
            for (key, nested) in object {
                validate_galgame_stage_outputs(
                    nested,
                    refs,
                    context,
                    owner_kind,
                    owner_id,
                    &format!("{path}.{key}"),
                )?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_galgame_stage_outputs(
                    item,
                    refs,
                    context,
                    owner_kind,
                    owner_id,
                    &format!("{path}[{index}]"),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}
