use std::collections::BTreeMap;

use serde_json::Value;

use super::{serialized_sha256, v1};
use crate::shooting_script::{ChapterShootingScriptV2, DirectorScriptPlan, source_hash};

/// Exact old-content → new-content mappings. Unmatched (already stale) identities stay stale.
#[derive(Debug, Default)]
pub struct HashUpgrades {
    replacements: BTreeMap<String, String>,
    plans: BTreeMap<i64, DirectorScriptPlan>,
}

impl HashUpgrades {
    pub fn register_plan(&mut self, id: i64, plan: DirectorScriptPlan) {
        self.plans.insert(id, plan);
    }

    pub fn insert(&mut self, old: String, new: String) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        if self
            .replacements
            .get(&old)
            .is_some_and(|existing| existing != &new)
        {
            return Err(format!("剧本升级遇到不一致的旧哈希映射: {old}"));
        }
        self.replacements.insert(old, new);
        Ok(())
    }

    pub fn mapped(&self, hash: &str) -> Option<&str> {
        self.replacements.get(hash).map(String::as_str)
    }

    pub fn upgrade_record(
        &mut self,
        value: &mut Value,
        stored_hash: &str,
    ) -> Result<String, String> {
        if value["schema_version"] != "chapter_shooting_script_v1" {
            let script: ChapterShootingScriptV2 =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            script.validate().map_err(|e| e.to_string())?;
            let hash = serialized_sha256(&script).map_err(|e| e.to_string())?;
            if hash != stored_hash {
                return Err("拍摄剧本内容哈希不一致".into());
            }
            return Ok(hash);
        }
        let old: v1::ChapterShootingScriptV1 =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if serialized_sha256(&old).map_err(|e| e.to_string())? != stored_hash {
            return Err("历史拍摄剧本内容哈希不一致，不能升级".into());
        }
        let script = old.upgrade().map_err(|e| e.to_string())?;
        script.validate().map_err(|e| e.to_string())?;
        let hash = serialized_sha256(&script).map_err(|e| e.to_string())?;
        self.record_script_change(&old, &script)?;
        *value = serde_json::to_value(script).map_err(|e| e.to_string())?;
        Ok(hash)
    }

    fn record_script_change(
        &mut self,
        old: &v1::ChapterShootingScriptV1,
        script: &ChapterShootingScriptV2,
    ) -> Result<(), String> {
        self.insert(
            serialized_sha256(old).map_err(|e| e.to_string())?,
            serialized_sha256(script).map_err(|e| e.to_string())?,
        )?;
        let plan = old
            .visual_plan_source
            .as_ref()
            .and_then(|source| self.plans.get(&source.plan_id));
        if let Some(plan) = plan {
            let old_shots = old
                .shot_content_fingerprints(plan)
                .map_err(|e| e.to_string())?;
            let new_shots = script
                .shot_content_fingerprints(plan)
                .map_err(|e| e.to_string())?;
            for (id, new) in new_shots {
                self.insert(old_shots[&id].clone(), new)?;
            }
        }
        let original = serde_json::to_value(old).map_err(|e| e.to_string())?;
        let upgraded = serde_json::to_value(script).map_err(|e| e.to_string())?;
        self.insert(
            crate::generation::canonical_json_sha256(&original).map_err(|e| e.to_string())?,
            crate::generation::canonical_json_sha256(&upgraded).map_err(|e| e.to_string())?,
        )?;
        self.insert(
            source_hash(&original).map_err(|e| e.to_string())?,
            source_hash(&upgraded).map_err(|e| e.to_string())?,
        )?;
        Ok(())
    }

    /// Only recognized script/projection objects are converted; arbitrary same-named fields
    /// and original request/response strings are never rewritten.
    pub fn upgrade_embedded(&mut self, value: &mut Value) -> Result<bool, String> {
        if value.get("schema_version").and_then(Value::as_str) == Some("chapter_shooting_script_v1")
        {
            let mut legacy = value.clone();
            if let Some(scenes) = legacy.get_mut("scenes").and_then(Value::as_array_mut) {
                for scene in scenes {
                    super::normalize_legacy_visual_constraint_points(scene);
                }
            }
            let old: v1::ChapterShootingScriptV1 =
                serde_json::from_value(legacy).map_err(|e| e.to_string())?;
            let script = old.upgrade().map_err(|e| e.to_string())?;
            self.record_script_change(&old, &script)?;
            *value = serde_json::to_value(script).map_err(|e| e.to_string())?;
            return Ok(true);
        } else if value.get("schema_version").and_then(Value::as_str)
            == Some("video_execution_projection_v2")
        {
            value["schema_version"] = Value::String("video_execution_projection_v3".into());
            value
                .as_object_mut()
                .expect("projection object")
                .remove("planning_quality");
            if let Some(scenes) = value.get_mut("scenes").and_then(Value::as_array_mut) {
                for scene in scenes {
                    if let Some(shots) = scene.get_mut("shots").and_then(Value::as_array_mut) {
                        for shot in shots {
                            remove_video_shot_estimates(shot)?;
                        }
                    }
                }
            }
            return Ok(true);
        } else if value.get("source_shot_id").is_some() && value.get("action_beats").is_some() {
            return remove_video_shot_estimates(value);
        } else if value.get("scene_identity").is_some()
            && value
                .get("shots")
                .and_then(Value::as_array)
                .is_some_and(|shots| {
                    shots
                        .iter()
                        .any(|shot| shot.get("estimated_natural_seconds").is_some())
                })
        {
            *value = serde_json::to_value(super::upgrade_scene(value).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }
        let mut changed = false;
        match value {
            Value::Array(items) => {
                for item in items {
                    changed |= self.upgrade_embedded(item)?;
                }
            }
            Value::Object(fields) => {
                for (key, item) in fields {
                    if !matches!(
                        key.as_str(),
                        "input_fingerprint_json"
                            | "request_json"
                            | "raw_response_json"
                            | "schema_upgrade"
                            | "original_input_fingerprint_json"
                            | "original_checkpoint"
                    ) {
                        changed |= self.upgrade_embedded(item)?;
                    }
                }
            }
            _ => {}
        }
        Ok(changed)
    }

    pub fn rewrite_references(&self, value: &mut Value) -> bool {
        let mut changed = false;
        match value {
            Value::Array(items) => {
                for item in items {
                    changed |= self.rewrite_references(item);
                }
            }
            Value::Object(fields) => {
                let progress_ref = fields.get("kind").and_then(Value::as_str)
                    == Some("chapter_shooting_script_progress");
                for (key, item) in fields {
                    if matches!(
                        key.as_str(),
                        "script_sha256"
                            | "shooting_script_sha256"
                            | "shooting_script_hash"
                            | "source_version_hash"
                    ) || (progress_ref && key == "revision")
                    {
                        changed |= self.rewrite_hash(item);
                    } else if key == "shot_fingerprints" {
                        if let Some(shots) = item.as_object_mut() {
                            for hash in shots.values_mut() {
                                changed |= self.rewrite_hash(hash);
                            }
                        }
                    } else if !matches!(
                        key.as_str(),
                        "input_fingerprint_json"
                            | "request_json"
                            | "raw_response_json"
                            | "schema_upgrade"
                            | "original_input_fingerprint_json"
                            | "original_checkpoint"
                    ) {
                        changed |= self.rewrite_references(item);
                    }
                }
            }
            _ => {}
        }
        changed
    }

    fn rewrite_hash(&self, value: &mut Value) -> bool {
        if let Some(new) = value.as_str().and_then(|old| self.mapped(old)) {
            *value = Value::String(new.into());
            return true;
        }
        false
    }
}

fn remove_video_shot_estimates(shot: &mut Value) -> Result<bool, String> {
    let fields = shot.as_object_mut().ok_or("视频投影镜头不是对象")?;
    let mut changed = false;
    for key in [
        "estimated_natural_seconds",
        "action_load",
        "segment_duration_secs",
        "recommended_segment_count",
        "minimum_readable_segment_count",
        "risk_flags",
    ] {
        changed |= fields.remove(key).is_some();
    }
    if let Some(beats) = fields.get_mut("action_beats").and_then(Value::as_array_mut) {
        for beat in beats {
            changed |= beat
                .as_object_mut()
                .ok_or("视频投影节拍不是对象")?
                .remove("natural_seconds")
                .is_some();
        }
    }
    Ok(changed)
}
