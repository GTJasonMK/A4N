use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use crate::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_SEGMENT_PLAN_V2, VideoSegmentPlanV2,
    video_segment_content_fingerprint,
};

/// 持久化规划只有 metadata.segments 一个事实来源。
pub fn video_segment_plan_from_metadata(
    metadata: &Value,
) -> Result<Vec<VideoSegmentPlanV2>, String> {
    let segments: Vec<VideoSegmentPlanV2> = serde_json::from_value(
        metadata
            .get("segments")
            .cloned()
            .ok_or("片段规划 metadata_json 缺少 segments")?,
    )
    .map_err(|error| format!("片段规划 segments 无效：{error}"))?;
    if segments.is_empty() {
        return Err("片段规划必须包含至少一个片段".into());
    }
    let mut keys = HashSet::new();
    let mut counts = HashMap::new();
    for (offset, segment) in segments.iter().enumerate() {
        if segment.schema_version != VIDEO_SEGMENT_PLAN_V2
            || usize::try_from(segment.segment_number).ok() != Some(offset + 1)
            || segment.source_shot_id.trim().is_empty()
            || segment.segment_index_in_shot <= 0
            || segment.segment_index_in_shot > segment.shot_segment_count
            || !keys.insert((&segment.source_shot_id, segment.segment_index_in_shot))
        {
            return Err(format!(
                "片段 {} 的版本、顺序或稳定镜头身份无效",
                segment.segment_number
            ));
        }
        *counts
            .entry(segment.source_shot_id.as_str())
            .or_insert(0usize) += 1;
    }
    for segment in &segments {
        if usize::try_from(segment.shot_segment_count).ok()
            != counts.get(segment.source_shot_id.as_str()).copied()
        {
            return Err(format!(
                "镜头 {} 的片段数量与 shot_segment_count 不一致",
                segment.source_shot_id
            ));
        }
    }
    Ok(segments)
}

pub fn video_segment_identities(segments: &[VideoSegmentPlanV2]) -> Result<Value, String> {
    segments
        .iter()
        .map(|segment| {
            Ok(json!({
                "source_shot_id": segment.source_shot_id,
                "segment_index_in_shot": segment.segment_index_in_shot,
                "segment_sha256": video_segment_content_fingerprint(segment)?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Value::Array)
}

pub fn validate_video_segment_plan_identity(
    identity: &Value,
    segments: &[VideoSegmentPlanV2],
) -> Result<(), String> {
    if identity.get("contract_epoch").and_then(Value::as_u64)
        != Some(VIDEO_EXECUTION_CONTRACT_EPOCH.into())
    {
        return Err(format!(
            "片段规划执行身份已过期，当前需要合同 {VIDEO_EXECUTION_CONTRACT_EPOCH}"
        ));
    }
    if identity.get("segment_identities") != Some(&video_segment_identities(segments)?) {
        return Err("片段规划执行身份不完整或内容指纹与 segments 不一致".into());
    }
    Ok(())
}
