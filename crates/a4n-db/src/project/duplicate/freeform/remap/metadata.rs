use super::super::FreeformCopyMaps;
use super::ids::remap_freeform_canvas_json_ids;
use crate::project::duplicate::ProjectDuplicateError;
use serde_json::Value;

pub(in crate::project::duplicate::freeform) fn remap_freeform_metadata(
    mut value: Value,
    maps: &FreeformCopyMaps,
) -> Result<Value, ProjectDuplicateError> {
    remap_freeform_canvas_json_ids(&mut value, maps, None)?;
    if let Some(canvas) = value.get_mut("canvas") {
        remap_freeform_canvas_json_ids(canvas, maps, None)?;
    }
    Ok(value)
}
