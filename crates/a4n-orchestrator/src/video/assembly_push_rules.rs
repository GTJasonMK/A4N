//! 成片推送（assembly_push）metadata 纯规则。

use a4n_core::models::ai::generated_video;
use chrono::Utc;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

/// 成片推送视频类型。
pub const ASSEMBLY_PUSH_VIDEO_TYPE: &str = "assembly_push";
/// 推送待审。
pub const ASSEMBLY_PUSH_STATUS_PENDING: &str = "pending";
/// 推送已接受。
pub const ASSEMBLY_PUSH_STATUS_ACCEPTED: &str = "accepted";
/// 推送已拒绝。
pub const ASSEMBLY_PUSH_STATUS_REJECTED: &str = "rejected";

/// 从 generated_video.metadata_json 解析出的推送批次纯字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyPushMetadata {
    pub push_index: i32,
    pub status: String,
    pub source_video_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct AssemblyPushRecord {
    pub video: generated_video::Model,
    pub status: String,
    pub push_index: i32,
    pub source_video_ids: Vec<i64>,
}

/// 是否为指定 workflow 下已完成的 assembly_push 行。
pub fn is_completed_assembly_push_row(
    video_type: &str,
    workflow_mode: Option<&str>,
    status: &str,
    expected_workflow_mode: &str,
) -> bool {
    video_type == ASSEMBLY_PUSH_VIDEO_TYPE
        && workflow_mode == Some(expected_workflow_mode)
        && status == "completed"
}

/// 解析 assembly_push 对象上的 status。
fn assembly_push_metadata_status(
    video_id: i64,
    metadata: &Map<String, Value>,
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

/// 从视频 metadata_json 解析 assembly_push 批次字段。
pub fn parse_assembly_push_metadata(
    video_id: i64,
    metadata_json: Option<&Value>,
) -> Result<AssemblyPushMetadata, String> {
    let metadata = metadata_json
        .and_then(|metadata| metadata.get("assembly_push"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("推送批次 #{video_id} 缺少 assembly_push metadata"))?;
    let push_index = metadata
        .get("push_index")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("推送批次 #{video_id} 缺少有效 push_index"))?;
    let status = assembly_push_metadata_status(video_id, metadata)?;
    let values = metadata
        .get("source_video_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("推送批次 #{video_id} 缺少 source_video_ids"))?;
    if values.is_empty() {
        return Err(format!("推送批次 #{video_id} 的 source_video_ids 不能为空"));
    }
    let mut seen = HashSet::new();
    let source_video_ids = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
                format!("推送批次 #{video_id} 的 source_video_ids[{index}] 必须是正整数")
            })?;
            if !seen.insert(id) {
                return Err(format!(
                    "推送批次 #{video_id} 的 source_video_ids 包含重复 ID：{id}"
                ));
            }
            Ok(id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AssemblyPushMetadata {
        push_index,
        status,
        source_video_ids,
    })
}

pub fn assembly_push_records(
    rows: &[generated_video::Model],
    workflow_mode: &str,
) -> Result<Vec<AssemblyPushRecord>, String> {
    let mut pushes = rows
        .iter()
        .filter(|row| {
            is_completed_assembly_push_row(
                &row.video_type,
                row.workflow_mode.as_deref(),
                &row.status,
                workflow_mode,
            )
        })
        .map(|row| {
            parse_assembly_push_metadata(row.id, row.metadata_json.as_ref()).map(|metadata| {
                AssemblyPushRecord {
                    video: row.clone(),
                    status: metadata.status,
                    push_index: metadata.push_index,
                    source_video_ids: metadata.source_video_ids,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    pushes.sort_by(|left, right| {
        left.push_index
            .cmp(&right.push_index)
            .then_with(|| left.video.id.cmp(&right.video.id))
    });
    Ok(pushes)
}

/// 校验成片 workflow_mode（trim 后白名单）。
pub fn validate_assembly_workflow_mode(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "boundary_frames" => Ok("boundary_frames"),
        "first_frame_chain" => Ok("first_frame_chain"),
        "reference_media" => Ok("reference_media"),
        "chat_video" => Ok("chat_video"),
        "storyboard" => Ok("storyboard"),
        "mixed" => Ok("mixed"),
        "" => Err("请选择视频生成流程后再拼接成片".to_string()),
        other => Err(format!("视频生成流程无效：{other}")),
    }
}

/// 是否为仍锁定成片前缀的推送状态（非 rejected）。
pub fn is_active_assembly_push_status(status: &str) -> bool {
    status != ASSEMBLY_PUSH_STATUS_REJECTED
}

/// 从推送批次列表收集仍生效的源片段 ID（跳过 rejected）。
pub fn active_assembly_push_source_ids<'a, I>(pushes: I) -> Vec<i64>
where
    I: IntoIterator<Item = (&'a str, &'a [i64])>,
{
    pushes
        .into_iter()
        .filter(|(status, _)| is_active_assembly_push_status(status))
        .flat_map(|(_, source_video_ids)| source_video_ids.iter().copied())
        .collect()
}

/// 最近一条非 rejected 推送在列表中的下标（从后往前）。
pub fn latest_non_rejected_push_index<'a, I>(statuses: I) -> Option<usize>
where
    I: IntoIterator<Item = &'a str>,
{
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    statuses
        .iter()
        .rposition(|status| is_active_assembly_push_status(status))
}

/// 已推送源 ID 必须等于当前成片列表前缀。
pub fn ensure_active_pushes_match_clip_prefix(
    clip_video_ids: &[i64],
    active_source_ids: &[i64],
) -> Result<(), String> {
    if active_source_ids.len() > clip_video_ids.len() {
        return Err("已推送片段数量超过当前成片片段数量，请检查成片列表".to_string());
    }
    for (index, source_id) in active_source_ids.iter().enumerate() {
        let clip_id = clip_video_ids[index];
        if clip_id != *source_id {
            return Err(format!(
                "已推送片段与当前成片列表不连续：第 {} 个已推送源片段应为 #{}，当前为 #{}",
                index + 1,
                source_id,
                clip_id
            ));
        }
    }
    Ok(())
}

/// 在待推送列表中按 end_video_id 选取闭区间结束下标；`None` 表示推到末尾。
pub fn selected_assembly_push_end_index(
    pending_video_ids: &[i64],
    end_video_id: Option<i64>,
) -> Result<usize, String> {
    if pending_video_ids.is_empty() {
        return Err("没有新的后续成片片段可推送".to_string());
    }
    if let Some(end_video_id) = end_video_id {
        let end_index = pending_video_ids
            .iter()
            .position(|id| *id == end_video_id)
            .ok_or_else(|| format!("推送结束片段 #{end_video_id} 不在可推送后续片段中"))?;
        return Ok(end_index);
    }
    Ok(pending_video_ids.len() - 1)
}

/// 插入位置不能落在已推送前缀内部（含开头）。
pub fn ensure_insert_position_not_inside_pushed_prefix(
    clip_video_ids: &[i64],
    active_source_ids: &[i64],
    insert_after_video_id: Option<i64>,
) -> Result<(), String> {
    ensure_active_pushes_match_clip_prefix(clip_video_ids, active_source_ids)?;
    if active_source_ids.is_empty() {
        return Ok(());
    }
    let Some(anchor_id) = insert_after_video_id else {
        return Err("已有成片片段推送后，不能再向已推送列表开头插入衔接片段".to_string());
    };
    let anchor_index = clip_video_ids
        .iter()
        .position(|id| *id == anchor_id)
        .ok_or_else(|| format!("插入位置视频 #{anchor_id} 不在当前成片片段中"))?;
    let pushed_last_index = active_source_ids.len() - 1;
    if anchor_index < pushed_last_index {
        return Err("不能在已推送成片片段中间插入衔接片段".to_string());
    }
    Ok(())
}

/// 更新（或创建）metadata_json 中的 assembly_push.status 与 status_updated_at。
pub fn assembly_push_metadata_with_status(base_metadata: Option<Value>, status: &str) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assembly_push_metadata_accepts_valid_batch() {
        let meta = parse_assembly_push_metadata(
            11,
            Some(&json!({
                "assembly_push": {
                    "status": "accepted",
                    "push_index": 1,
                    "source_video_ids": [1, 2]
                }
            })),
        )
        .unwrap();
        assert_eq!(meta.push_index, 1);
        assert_eq!(meta.status, ASSEMBLY_PUSH_STATUS_ACCEPTED);
        assert_eq!(meta.source_video_ids, vec![1, 2]);
        assert!(is_completed_assembly_push_row(
            ASSEMBLY_PUSH_VIDEO_TYPE,
            Some("first_frame_chain"),
            "completed",
            "first_frame_chain"
        ));
    }

    #[test]
    fn parse_assembly_push_metadata_rejects_malformed() {
        let missing_status = parse_assembly_push_metadata(
            100,
            Some(&json!({
                "assembly_push": {
                    "push_index": 4,
                    "source_video_ids": [4]
                }
            })),
        )
        .unwrap_err();
        assert!(missing_status.contains("缺少 assembly_push.status"));

        let invalid_status = parse_assembly_push_metadata(
            101,
            Some(&json!({
                "assembly_push": {
                    "status": "done",
                    "push_index": 5,
                    "source_video_ids": [5]
                }
            })),
        )
        .unwrap_err();
        assert!(invalid_status.contains("assembly_push.status 无效"));

        let empty_sources = parse_assembly_push_metadata(
            102,
            Some(&json!({
                "assembly_push": {
                    "status": "pending",
                    "push_index": 6,
                    "source_video_ids": []
                }
            })),
        )
        .unwrap_err();
        assert!(empty_sources.contains("source_video_ids 不能为空"));
    }

    #[test]
    fn validate_assembly_workflow_mode_accepts_known_modes() {
        assert_eq!(
            validate_assembly_workflow_mode("boundary_frames"),
            Ok("boundary_frames")
        );
        assert_eq!(
            validate_assembly_workflow_mode(" first_frame_chain "),
            Ok("first_frame_chain")
        );
        assert_eq!(
            validate_assembly_workflow_mode("reference_media"),
            Ok("reference_media")
        );
        assert_eq!(
            validate_assembly_workflow_mode("chat_video"),
            Ok("chat_video")
        );
        assert!(validate_assembly_workflow_mode(" ").is_err());
        assert!(validate_assembly_workflow_mode("unknown").is_err());
    }

    #[test]
    fn active_source_ids_skip_rejected_and_prefix_rules() {
        let active = active_assembly_push_source_ids([
            (ASSEMBLY_PUSH_STATUS_ACCEPTED, &[1_i64, 2][..]),
            (ASSEMBLY_PUSH_STATUS_REJECTED, &[9][..]),
            (ASSEMBLY_PUSH_STATUS_PENDING, &[4][..]),
        ]);
        assert_eq!(active, vec![1, 2, 4]);

        ensure_active_pushes_match_clip_prefix(&[1, 2, 3], &[1, 2]).unwrap();
        let gap = ensure_active_pushes_match_clip_prefix(&[1, 2], &[1, 3]).unwrap_err();
        assert!(gap.contains("不连续"));
        let overflow = ensure_active_pushes_match_clip_prefix(&[1, 2], &[1, 2, 3]).unwrap_err();
        assert!(overflow.contains("超过当前成片片段数量"));
    }

    #[test]
    fn selected_end_index_and_insert_prefix_guards() {
        assert_eq!(
            selected_assembly_push_end_index(&[3, 4, 5], Some(4)).unwrap(),
            1
        );
        assert_eq!(
            selected_assembly_push_end_index(&[3, 4, 5], None).unwrap(),
            2
        );
        assert!(
            selected_assembly_push_end_index(&[3, 4], Some(2))
                .unwrap_err()
                .contains("不在可推送后续片段中")
        );

        ensure_insert_position_not_inside_pushed_prefix(&[1, 2, 3], &[1, 2], Some(2)).unwrap();
        assert!(
            ensure_insert_position_not_inside_pushed_prefix(&[1, 2, 3], &[1, 2], None)
                .unwrap_err()
                .contains("开头插入")
        );
        assert!(
            ensure_insert_position_not_inside_pushed_prefix(&[1, 2, 3], &[1, 2], Some(1))
                .unwrap_err()
                .contains("中间插入")
        );
    }
}
