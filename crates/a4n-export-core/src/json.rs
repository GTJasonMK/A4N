use serde::Serialize;

use crate::contracts::{ExportRenderJsonInput, ExportRenderJsonOutput};
use crate::errors::ExportRenderError;

pub fn render_json(
    input: ExportRenderJsonInput,
) -> Result<ExportRenderJsonOutput, ExportRenderError> {
    let content = if input.pretty {
        serde_json::to_string_pretty(&input.payload)?
    } else {
        serde_json::to_string(&input.payload)?
    };
    Ok(ExportRenderJsonOutput {
        kind: input.kind,
        content,
        content_type: "application/json".to_string(),
    })
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
