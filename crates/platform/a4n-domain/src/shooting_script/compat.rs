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

fn normalize_legacy_visual_constraint_points(scene: &mut Value) {
    let Some(shots) = scene.get_mut("shots").and_then(Value::as_array_mut) else {
        return;
    };
    for shot in shots {
        let shot_fields = shot.as_object().cloned().unwrap_or_default();
        let Some(fields) = shot
            .get_mut("visual_constraints")
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        for field in [
            "entity_events",
            "camera_events",
            "sound_events",
            "technique_events",
            "continuity_notes",
            "transition_to_next",
        ] {
            fields.remove(field);
        }
        let start_ms = shot_fields
            .get("planned_start_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let end_ms = shot_fields
            .get("planned_end_ms")
            .and_then(Value::as_u64)
            .filter(|end| *end > start_ms)
            .unwrap_or(start_ms.saturating_add(1_000));
        fields
            .entry("frame")
            .or_insert_with(|| serde_json::json!({"width": 1920, "height": 1080}));
        fields.entry("segment_id").or_insert_with(|| {
            shot_fields
                .get("visual_shot_id")
                .and_then(Value::as_str)
                .map(|value| Value::String(value.into()))
                .or_else(|| shot_fields.get("shot_id").cloned())
                .unwrap_or_else(|| Value::String("legacy-shot".into()))
        });
        fields.entry("title").or_insert_with(|| {
            shot_fields
                .get("title")
                .cloned()
                .unwrap_or_else(|| Value::String("历史镜头".into()))
        });
        fields
            .entry("start_ms")
            .or_insert_with(|| Value::from(start_ms));
        fields
            .entry("end_ms")
            .or_insert_with(|| Value::from(end_ms));
        fields.entry("prompt").or_insert_with(|| {
            shot_fields
                .get("narrative_purpose")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new()))
        });
        fields
            .entry("scene_binding")
            .or_insert_with(|| serde_json::json!({"kind": "scene", "scene_profile_id": 1}));
        fields.entry("end_frame").or_insert(Value::Null);
        for key in ["keyframes", "end_frame"] {
            if let Some(points) = fields.get_mut(key).and_then(Value::as_array_mut) {
                for point in points {
                    normalize_legacy_visual_point(point);
                }
            } else if let Some(point) = fields.get_mut(key) {
                normalize_legacy_visual_point(point);
            }
        }
    }
}

fn normalize_legacy_visual_point(point: &mut Value) {
    let Some(fields) = point.as_object_mut() else {
        return;
    };
    let description = fields
        .get("description")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            fields
                .get("camera")
                .and_then(|camera| camera.get("framing_note"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            fields
                .get("label")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default();
    if !fields.contains_key("description") {
        fields.insert("description".into(), Value::String(description));
    }
    fields.entry("image_id").or_insert(Value::Null);
    fields
        .entry("reference_image_ids")
        .or_insert_with(|| Value::Array(Vec::new()));
    for field in [
        "interpolation_to_next",
        "camera",
        "entity_states",
        "relations",
    ] {
        fields.remove(field);
    }
}

fn upgrade_scene(value: &Value) -> Result<super::ShootingSceneV2, serde_json::Error> {
    let mut value = value.clone();
    normalize_legacy_visual_constraint_points(&mut value);
    let legacy: v1::ShootingSceneV1 = serde_json::from_value(value)?;
    let mut value = serde_json::to_value(legacy)?;
    remove_scene_estimates(&mut value);
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests;
