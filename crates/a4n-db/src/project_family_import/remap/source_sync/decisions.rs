use super::super::super::{ImportIdMaps, ProjectFamilyImportError};
use super::super::{json_object, remap_object_id};
use super::{
    galgame::remap_regenerate_scene_payload,
    reference_payload::{
        remap_anchor_mappings, remap_refresh_spec_payload, remap_replacement_segment_payload,
    },
};
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use serde_json::Value;

pub(in crate::project_family_import) fn remap_source_sync_decision_payload(
    strategy: &str,
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(value, label)?;
    let strategy = SourceSyncResolutionStrategy::parse(strategy).ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("同步决策 strategy 无效: {strategy}"))
    })?;
    match strategy {
        SourceSyncResolutionStrategy::KeepLocal
        | SourceSyncResolutionStrategy::Detach
        | SourceSyncResolutionStrategy::DeleteLocal => {}
        SourceSyncResolutionStrategy::Manual => {
            remap_object_id(
                &mut object,
                "prompt_artifact_id",
                &maps.prompt_artifact_ids,
                label,
            )?;
        }
        SourceSyncResolutionStrategy::RebindAnchor => {
            remap_object_id(
                &mut object,
                "generation_spec_id",
                &maps.generation_spec_ids,
                label,
            )?;
            remap_anchor_mappings(&mut object, maps, label)?;
        }
        SourceSyncResolutionStrategy::AdoptSource | SourceSyncResolutionStrategy::RebuildPrompt => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
        }
        SourceSyncResolutionStrategy::RegenerateScene => {
            remap_regenerate_scene_payload(&mut object, maps, label)?;
        }
        SourceSyncResolutionStrategy::CreateReplacement => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
            remap_replacement_segment_payload(&mut object, maps, label)?;
        }
    }
    Ok(Value::Object(object))
}
