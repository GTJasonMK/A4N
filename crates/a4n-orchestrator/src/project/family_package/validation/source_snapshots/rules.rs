use std::collections::HashMap;

use serde_json::Value;

use crate::project::required_json_i64_field;

use super::super::hash_json;
use super::manifest::SourceSnapshotManifestInfo;

pub(in crate::project::family_package::validation) fn parse_chapter_scope_json(
    value: &Value,
    label: &str,
) -> Result<(i32, i32), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} 必须是 JSON 对象"))?;
    let start = object
        .get("start")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.start 必须是整数"))?;
    let end = object
        .get("end")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.end 必须是整数"))?;
    if start <= 0 || end < start || start > i32::MAX as i64 || end > i32::MAX as i64 {
        return Err(format!("{label} 范围无效: start={start}, end={end}"));
    }
    Ok((
        i32::try_from(start).map_err(|_| format!("{label}.start 超出 i32 范围: {start}"))?,
        i32::try_from(end).map_err(|_| format!("{label}.end 超出 i32 范围: {end}"))?,
    ))
}

pub(super) fn validate_snapshot_payload_identity(
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
    snapshot_id: i64,
    root_project_id: i64,
    row_id: i64,
    kind: &str,
    source_id: i64,
    payload_json: &Value,
    payload_hash: &str,
    row_label: &str,
    row_id_field: &str,
    kind_field: &str,
    source_id_field: &str,
    is_valid_kind: fn(&str) -> bool,
) -> Result<(), String> {
    let snapshot = snapshots.get(&snapshot_id).ok_or_else(|| {
        format!(
            "项目族导入包{row_label}引用了不存在的源文本快照: {row_id_field}={row_id}, snapshot_id={snapshot_id}"
        )
    })?;
    if root_project_id != snapshot.root_project_id {
        return Err(format!(
            "项目族导入包{row_label}根项目不一致: {row_id_field}={row_id}, root_project_id={root_project_id}"
        ));
    }
    if !is_valid_kind(kind) {
        return Err(format!(
            "项目族导入包{row_label}类型无效: {row_id_field}={row_id}, {kind_field}={kind}"
        ));
    }
    let payload = payload_json.as_object().ok_or_else(|| {
        format!("项目族导入包{row_label} payload_json 必须是 JSON 对象: {row_id_field}={row_id}")
    })?;
    let payload_id = required_json_i64_field(payload, "id", &format!("{row_label} payload_json"))?;
    if payload_id != source_id {
        return Err(format!(
            "项目族导入包{row_label} payload_json.id 与 {source_id_field} 不一致: {row_id_field}={row_id}, payload_id={payload_id}, {source_id_field}={source_id}"
        ));
    }
    let payload_project_id =
        required_json_i64_field(payload, "project_id", &format!("{row_label} payload_json"))?;
    if payload_project_id != root_project_id {
        return Err(format!(
            "项目族导入包{row_label} payload_json.project_id 与 root_project_id 不一致: {row_id_field}={row_id}, payload_project_id={payload_project_id}, root_project_id={root_project_id}"
        ));
    }
    let expected_hash = hash_json(payload_json);
    if payload_hash != expected_hash {
        return Err(format!(
            "项目族导入包{row_label} payload_hash 与 payload_json 不一致: {row_id_field}={row_id}"
        ));
    }
    Ok(())
}

pub(super) fn is_valid_snapshot_outline_kind(kind: &str) -> bool {
    matches!(kind, "part" | "chapter")
}

pub(super) fn is_valid_snapshot_profile_kind(kind: &str) -> bool {
    matches!(kind, "character" | "object" | "scene")
}
