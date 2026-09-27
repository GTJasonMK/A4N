//! A completed v1 call keeps its original wire evidence beside its upgraded parsed output.

use serde_json::{Value, json};

use super::HashUpgrades;
use crate::generation::canonical_json_sha256;

pub const TIMING_UPGRADE_KEY: &str = "shooting_timing_v2";

pub fn is_timing_upgrade_step(step: &str) -> bool {
    step.starts_with("shooting_design_shot:") || step.starts_with("video_segment_plan.shot.")
}

pub fn upgrade_completed_checkpoint(
    step: &str,
    input: Option<&Value>,
    input_hash: Option<&str>,
    checkpoint: &mut Value,
    artifacts: &mut Option<Value>,
    hashes: &mut HashUpgrades,
) -> Result<bool, String> {
    if !is_timing_upgrade_step(step) {
        let changed = hashes.upgrade_embedded(checkpoint)?;
        return Ok(hashes.rewrite_references(checkpoint) | changed);
    }
    let input = input.ok_or_else(|| format!("{step} 的已完成断点缺少输入指纹"))?;
    let input_hash = input_hash.ok_or_else(|| format!("{step} 的已完成断点缺少输入指纹哈希"))?;
    if canonical_json_sha256(input).map_err(|e| e.to_string())? != input_hash {
        return Err(format!("{step} 的历史调用输入指纹损坏"));
    }
    if artifacts
        .as_ref()
        .and_then(|value| value.get(TIMING_UPGRADE_KEY))
        .is_some()
    {
        validate_checkpoint_upgrade(checkpoint, artifacts.as_ref().unwrap())?;
        return Ok(false);
    }
    let input_text = input.to_string();
    if !input_text.contains("estimated_seconds")
        && !input_text.contains("natural_seconds")
        && !input_text.contains("video_execution_projection_v2")
        && !input_text.contains("ShootingSceneV1")
    {
        return Ok(false);
    }
    let original = checkpoint.clone();
    let parsed = checkpoint
        .get_mut("parsed_output")
        .ok_or_else(|| format!("{step} 缺少 parsed_output"))?;
    hashes.upgrade_embedded(parsed)?;
    hashes.rewrite_references(parsed);
    let mut upgraded_input = input.clone();
    hashes.upgrade_embedded(&mut upgraded_input)?;
    hashes.rewrite_references(&mut upgraded_input);
    let receipt = json!({
        "version": 1,
        "original_input_fingerprint_json": input,
        "original_input_fingerprint_sha256": input_hash,
        "upgraded_input_fingerprint_json": upgraded_input,
        "upgraded_input_fingerprint_sha256": canonical_json_sha256(&upgraded_input).map_err(|e| e.to_string())?,
        "original_checkpoint": original,
        "original_checkpoint_sha256": canonical_json_sha256(&original).map_err(|e| e.to_string())?,
        "upgraded_parsed_output_sha256": canonical_json_sha256(&checkpoint["parsed_output"]).map_err(|e| e.to_string())?,
    });
    let artifacts = artifacts.get_or_insert_with(|| json!({}));
    artifacts
        .as_object_mut()
        .ok_or("断点 artifacts 必须是对象")?
        .insert(TIMING_UPGRADE_KEY.into(), receipt);
    validate_checkpoint_upgrade(checkpoint, artifacts)?;
    Ok(true)
}

pub fn validate_checkpoint_upgrade(checkpoint: &Value, artifacts: &Value) -> Result<(), String> {
    let receipt = artifacts
        .get(TIMING_UPGRADE_KEY)
        .ok_or("缺少剧本合同升级回执")?;
    if receipt.get("version").and_then(Value::as_u64) != Some(1) {
        return Err("不支持的剧本断点升级回执版本".into());
    }
    let input = &receipt["original_input_fingerprint_json"];
    if canonical_json_sha256(input).map_err(|e| e.to_string())?
        != receipt["original_input_fingerprint_sha256"]
    {
        return Err("剧本断点升级回执的原始输入哈希不一致".into());
    }
    let original = &receipt["original_checkpoint"];
    if canonical_json_sha256(&receipt["upgraded_input_fingerprint_json"])
        .map_err(|e| e.to_string())?
        != receipt["upgraded_input_fingerprint_sha256"]
    {
        return Err("剧本断点升级回执的当前输入哈希不一致".into());
    }
    if canonical_json_sha256(original).map_err(|e| e.to_string())?
        != receipt["original_checkpoint_sha256"]
    {
        return Err("剧本断点升级回执的原始输出哈希不一致".into());
    }
    if checkpoint["response_content"] != original["response_content"]
        || checkpoint["usage"] != original["usage"]
        || checkpoint["finish_reason"] != original["finish_reason"]
    {
        return Err("剧本断点原始响应或 usage 被改写".into());
    }
    let raw = original["response_content"]
        .as_str()
        .ok_or("历史断点缺少原始响应")?;
    let mut from_raw: Value =
        serde_json::from_str(raw).map_err(|e| format!("历史断点原始 JSON 无效: {e}"))?;
    let mut from_saved = original["parsed_output"].clone();
    let mut hashes = HashUpgrades::default();
    hashes.upgrade_embedded(&mut from_raw)?;
    hashes.upgrade_embedded(&mut from_saved)?;
    // Segment-plan checkpoints contain backend-derived fields and are revalidated by
    // their existing projection validator; shooting calls store the exact parsed scene.
    if canonical_json_sha256(&checkpoint["parsed_output"]).map_err(|e| e.to_string())?
        != receipt["upgraded_parsed_output_sha256"]
    {
        return Err("历史断点升级后的结构化输出不一致".into());
    }
    if from_raw.get("shots").is_some() && from_raw != from_saved {
        return Err("历史拍摄设计的原始响应与已保存输出不一致".into());
    }
    Ok(())
}
