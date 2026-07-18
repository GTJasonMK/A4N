use a4n_core::models::generation::generation_spec;
use serde_json::Value;

use crate::project_family_import::{ImportIdMaps, ProjectFamilyImportError};

use super::super::json_ids::json_object;
use super::galgame::{
    remap_galgame_background_params, remap_galgame_cg_params, remap_galgame_portrait_params,
};

pub fn remap_generation_spec_params_json(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(&spec.params_json, "生成意图 params_json")?;
    match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
        ("galgame_background", "galgame_scene") => {
            remap_galgame_background_params(spec, &mut object, maps)?;
        }
        ("galgame_cg", "galgame_line") => {
            remap_galgame_cg_params(spec, &mut object, maps)?;
        }
        ("portrait_expression", "character_profile") => {
            remap_galgame_portrait_params(spec, &mut object, maps)?;
        }
        _ => {}
    }
    Ok(Value::Object(object))
}
