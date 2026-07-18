use super::super::contracts::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_STATUS_REJECTED,
    ASSEMBLY_PUSH_VIDEO_TYPE, AssemblyPushRow,
};
use a4n_core::models::ai::generated_video;
use chrono::Utc;
use serde_json::{Value, json};
use std::collections::HashSet;

pub(in crate::actions::video_records) fn assembly_push_rows(
    rows: &[generated_video::Model],
    workflow_mode: &str,
) -> Result<Vec<AssemblyPushRow>, String> {
    let mut pushes = rows
        .iter()
        .filter(|row| {
            row.video_type == ASSEMBLY_PUSH_VIDEO_TYPE
                && row.workflow_mode.as_deref() == Some(workflow_mode)
                && row.status == "completed"
        })
        .map(parse_assembly_push_row)
        .collect::<Result<Vec<_>, _>>()?;
    pushes.sort_by(|left, right| {
        left.push_index
            .cmp(&right.push_index)
            .then_with(|| left.video.id.cmp(&right.video.id))
    });
    Ok(pushes)
}

fn parse_assembly_push_row(row: &generated_video::Model) -> Result<AssemblyPushRow, String> {
    let metadata = row
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_push"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("推送批次 #{} 缺少 assembly_push metadata", row.id))?;
    let push_index = metadata
        .get("push_index")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("推送批次 #{} 缺少有效 push_index", row.id))?;
    let status = assembly_push_metadata_status(row.id, metadata)?;
    let values = metadata
        .get("source_video_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("推送批次 #{} 缺少 source_video_ids", row.id))?;
    if values.is_empty() {
        return Err(format!("推送批次 #{} 的 source_video_ids 不能为空", row.id));
    }
    let mut seen = HashSet::new();
    let source_video_ids = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
                format!(
                    "推送批次 #{} 的 source_video_ids[{index}] 必须是正整数",
                    row.id
                )
            })?;
            if !seen.insert(id) {
                return Err(format!(
                    "推送批次 #{} 的 source_video_ids 包含重复 ID：{id}",
                    row.id
                ));
            }
            Ok(id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AssemblyPushRow {
        video: row.clone(),
        status,
        push_index,
        source_video_ids,
    })
}

fn assembly_push_metadata_status(
    video_id: i64,
    metadata: &serde_json::Map<String, Value>,
) -> Result<String, String> {
    let status = metadata
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{video_id} 缺少 assembly_push.status"))?;
    match status {
        ASSEMBLY_PUSH_STATUS_PENDING
        | ASSEMBLY_PUSH_STATUS_ACCEPTED
        | ASSEMBLY_PUSH_STATUS_REJECTED => Ok(status.to_string()),
        other => Err(format!(
            "推送批次 #{video_id} 的 assembly_push.status 无效: {other}"
        )),
    }
}

pub(in crate::actions::video_records::assembly) fn assembly_push_status(
    video: &generated_video::Model,
) -> Result<String, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_push"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("推送批次 #{} 缺少 assembly_push.status", video.id))?;
    assembly_push_metadata_status(video.id, metadata)
}

pub(in crate::actions::video_records::assembly) fn assembly_push_metadata_with_status(
    base_metadata: Option<Value>,
    status: &str,
) -> Value {
    let mut metadata = match base_metadata {
        Some(Value::Object(map)) => Value::Object(map),
        Some(value) => json!({ "metadata": value }),
        None => json!({}),
    };
    let Some(map) = metadata.as_object_mut() else {
        return json!({ "assembly_push": { "status": status } });
    };
    let entry = map
        .entry("assembly_push".to_string())
        .or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    if let Some(push) = entry.as_object_mut() {
        push.insert("status".to_string(), json!(status));
        push.insert(
            "status_updated_at".to_string(),
            json!(Utc::now().to_rfc3339()),
        );
    }
    metadata
}
