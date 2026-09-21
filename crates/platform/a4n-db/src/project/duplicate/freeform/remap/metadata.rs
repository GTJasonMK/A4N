use super::super::FreeformCopyMaps;
use super::ids::remap_freeform_canvas_json_ids;
use crate::project::duplicate::ProjectDuplicateError;
use a4n_domain::clear_canvas_transfer_runtime_identity;
use serde_json::Value;

pub(in crate::project::duplicate::freeform) fn remap_freeform_metadata(
    mut value: Value,
    maps: &FreeformCopyMaps,
) -> Result<Value, ProjectDuplicateError> {
    remap_freeform_metadata_object(&mut value, maps)?;
    clear_canvas_transfer_runtime_identity(&mut value);
    Ok(value)
}

fn remap_freeform_metadata_object(
    value: &mut Value,
    maps: &FreeformCopyMaps,
) -> Result<(), ProjectDuplicateError> {
    remap_freeform_canvas_json_ids(value, maps, None)?;
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    for key in [
        "canvas",
        "source_material",
        "params",
        "node_config",
        "metadata",
    ] {
        if let Some(nested) = object.get_mut(key) {
            remap_freeform_metadata_object(nested, maps)?;
        }
    }
    Ok(())
}
