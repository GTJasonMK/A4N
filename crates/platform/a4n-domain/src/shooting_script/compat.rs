//! Explicit version upgrades at storage/import/checkpoint boundaries.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub mod checkpoint;
mod hashes;
pub mod v1;
pub use hashes::HashUpgrades;

pub fn serialized_sha256(value: &impl Serialize) -> Result<String, serde_json::Error> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn remove_scene_estimates(scene: &mut Value) {
    if let Some(shots) = scene.get_mut("shots").and_then(Value::as_array_mut) {
        for shot in shots {
            if let Some(fields) = shot.as_object_mut() {
                fields.remove("estimated_natural_seconds");
                if let Some(beats) = fields
                    .get_mut("performance_beats")
                    .and_then(Value::as_array_mut)
                {
                    for beat in beats {
                        if let Some(fields) = beat.as_object_mut() {
                            fields.remove("estimated_seconds");
                        }
                    }
                }
            }
        }
    }
}

fn remove_script_estimates(script: &mut Value) {
    if let Some(scenes) = script.get_mut("scenes").and_then(Value::as_array_mut) {
        for scene in scenes {
            remove_scene_estimates(scene);
        }
    }
}

fn upgrade_scene(value: &Value) -> Result<super::ShootingSceneV2, serde_json::Error> {
    let legacy: v1::ShootingSceneV1 = serde_json::from_value(value.clone())?;
    let mut value = serde_json::to_value(legacy)?;
    remove_scene_estimates(&mut value);
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests;
