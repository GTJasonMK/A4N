//! 源快照载荷身份 Host 适配：查清单后委托 orch 纯规则。

use std::collections::HashMap;

use serde_json::Value;

use super::manifest::SourceSnapshotManifestInfo;

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
    crate::project::validate_snapshot_payload_identity(
        snapshot.root_project_id,
        snapshot_id,
        root_project_id,
        row_id,
        kind,
        source_id,
        payload_json,
        payload_hash,
        row_label,
        row_id_field,
        kind_field,
        source_id_field,
        is_valid_kind,
    )
}
