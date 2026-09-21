use serde::Serialize;

use crate::errors::ExportRenderError;
use a4n_export_contract::ExportRenderJsonInput;

pub fn render_json(input: ExportRenderJsonInput) -> Result<String, ExportRenderError> {
    if input.pretty {
        Ok(serde_json::to_string_pretty(&input.payload)?)
    } else {
        Ok(serde_json::to_string(&input.payload)?)
    }
}

pub fn render_json_manifest_pretty(
    payload: &impl Serialize,
    label: impl Into<String>,
) -> Result<Vec<u8>, ExportRenderError> {
    serde_json::to_vec_pretty(payload).map_err(|source| ExportRenderError::JsonManifest {
        label: label.into(),
        source,
    })
}
