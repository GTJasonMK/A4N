use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::generation::generation_spec;
use a4n_core::source_spans::source_text_hash;
use a4n_orchestrator::canvas::FreeformCanvasGenerationSpecUpsertInput;
use a4n_orchestrator::project::SOURCE_MODE_FREEFORM;
use serde_json::{Value, json};

pub(in crate::actions::canvas_workflow) struct FreeformCanvasSpecInput<'a> {
    pub(in crate::actions::canvas_workflow) project_id: i64,
    pub(in crate::actions::canvas_workflow) graph_id: i64,
    pub(in crate::actions::canvas_workflow) node_id: i64,
    pub(in crate::actions::canvas_workflow) canvas_text_asset_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) target_kind: &'a str,
    pub(in crate::actions::canvas_workflow) target_entity_kind: &'a str,
    pub(in crate::actions::canvas_workflow) status: &'a str,
    pub(in crate::actions::canvas_workflow) source_material_json: Value,
    pub(in crate::actions::canvas_workflow) params_json: Value,
}

pub(in crate::actions::canvas_workflow) async fn upsert_freeform_canvas_generation_spec(
    state: &AppState,
    input: FreeformCanvasSpecInput<'_>,
) -> Result<Option<generation_spec::Model>, String> {
    let project = state
        .orchestrator
        .project_by_id(input.project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {}", input.project_id))?;
    if project.source_mode != SOURCE_MODE_FREEFORM {
        return Ok(None);
    }

    let source_version_hash = sha256_hex_value(&input.source_material_json)?;
    let params_json = json!({
        "canvas_graph_id": input.graph_id,
        "canvas_node_id": input.node_id,
        "source_material": input.source_material_json,
        "params": input.params_json,
    });
    let spec = state
        .orchestrator
        .upsert_freeform_canvas_generation_spec_record(FreeformCanvasGenerationSpecUpsertInput {
            project_id: input.project_id,
            target_entity_id: input.node_id,
            canvas_text_asset_id: input.canvas_text_asset_id,
            target_kind: input.target_kind.to_string(),
            target_entity_kind: input.target_entity_kind.to_string(),
            status: input.status.to_string(),
            source_version_hash,
            params_json,
        })
        .await
        .map_err(app_error_message)?;
    Ok(Some(spec))
}

pub(in crate::actions::canvas_workflow) async fn mark_canvas_generation_spec_status(
    state: &AppState,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), String> {
    state
        .orchestrator
        .mark_canvas_generation_spec_status_record(generation_spec_id, status)
        .await
        .map_err(app_error_message)
}

fn sha256_hex_value(value: &Value) -> Result<String, String> {
    let serialized = serde_json::to_string(value).map_err(|err| err.to_string())?;
    Ok(source_text_hash(&serialized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_version_hash_uses_serialized_json_text() {
        let value = json!({
            "source": "画布 & 文本",
            "selection": {"end": 12, "start": 3},
        });
        let serialized = serde_json::to_string(&value).unwrap();

        assert_eq!(
            sha256_hex_value(&value).unwrap(),
            source_text_hash(&serialized)
        );
    }
}
