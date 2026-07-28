//! 项目族/视频画布导入包纯校验与哈希工具（无 DB、无 Host）。

use std::collections::HashSet;
use std::path::Path;

use a4n_core::models::media::media_asset;
use a4n_core::models::project::novel_project;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM};

/// 字节 SHA-256 十六进制。
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// JSON 规范序列化后的 SHA-256。
pub fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    sha256_bytes(&bytes)
}

/// 文本 SHA-256。
pub fn hash_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

/// 解析章节范围 JSON：`{"start":n,"end":m}`，start≥1 且 end≥start。
pub fn parse_chapter_scope_json(value: &Value, label: &str) -> Result<(i32, i32), String> {
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

/// 源快照大纲 kind 是否合法。
pub fn is_valid_snapshot_outline_kind(kind: &str) -> bool {
    matches!(kind, "part" | "chapter")
}

/// 源快照档案 kind 是否合法。
pub fn is_valid_snapshot_profile_kind(kind: &str) -> bool {
    matches!(kind, "character" | "object" | "scene")
}

pub use super::json_field_rules::required_json_i64_field;

/// 校验快照 payload 身份与哈希（调用方已解析 snapshot 根项目 id）。
pub fn validate_snapshot_payload_identity(
    snapshot_root_project_id: i64,
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
    if root_project_id != snapshot_root_project_id {
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
    let _ = snapshot_id;
    Ok(())
}

/// freeform 视频项目结构约束。
pub fn validate_freeform_video_project(
    project: &novel_project::Model,
    label: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err(format!("{label}必须是 source_mode=freeform 的 video 项目"));
    }
    if project.root_project_id.is_some()
        || project.parent_project_id.is_some()
        || project.source_snapshot_id.is_some()
        || project.derived_project_status.is_some()
    {
        return Err(format!("{label}不能包含文本派生项目引用字段"));
    }
    Ok(())
}

/// ID 列表唯一性（视频画布包文案）。
pub fn validate_unique_ids<I>(ids: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = i64>,
{
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("视频画布项目包 ID 重复: {label}={id}"));
        }
    }
    Ok(())
}

/// 导入包（family package）ID 唯一性。
pub fn validate_import_unique_ids<I>(ids: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = i64>,
{
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("导入包包含重复 {label}: {id}"));
        }
    }
    Ok(())
}

/// 导入包复合 key 字符串唯一性。
pub fn validate_import_unique_key_strings<I>(keys: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    for key in keys {
        if !seen.insert(key.clone()) {
            return Err(format!("导入包包含重复 {label}: {key}"));
        }
    }
    Ok(())
}

/// 解析 `canvas_graph:{id}:node:{id}` 缓存键。
pub fn parse_canvas_cache_key(value: &str) -> Option<(i64, i64)> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        return Some((graph_id, node_id));
    }
    None
}

/// 媒体资产路径字段名常量（与导入包约定一致）。
pub const MEDIA_FILE_FIELD_FILE: &str = "file_path";
pub const MEDIA_FILE_FIELD_THUMBNAIL: &str = "thumbnail_path";

/// 按字段取媒体资产路径。
pub fn media_asset_path<'a>(
    asset: &'a media_asset::Model,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match field {
        MEDIA_FILE_FIELD_FILE => Ok(asset.file_path.as_deref()),
        MEDIA_FILE_FIELD_THUMBNAIL => Ok(asset.thumbnail_path.as_deref()),
        _ => Err(format!("媒体文件字段无效: {field}")),
    }
}

/// 媒体包路径必填扩展名（小写）。
pub fn required_package_extension(path: &str) -> Result<String, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("媒体包路径缺少扩展名: {path}"))
}

/// 项目族包内媒体文件索引键。
pub fn media_file_key(media_asset_id: i64, field: &str) -> (i64, String) {
    (media_asset_id, field.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chapter_scope_parses_and_rejects_invalid() {
        assert_eq!(
            parse_chapter_scope_json(&json!({"start": 1, "end": 3}), "scope").unwrap(),
            (1, 3)
        );
        assert!(parse_chapter_scope_json(&json!({"start": 0, "end": 1}), "scope").is_err());
        assert!(parse_chapter_scope_json(&json!({"start": 2, "end": 1}), "scope").is_err());
    }

    #[test]
    fn snapshot_kinds() {
        assert!(is_valid_snapshot_outline_kind("part"));
        assert!(!is_valid_snapshot_outline_kind("foo"));
        assert!(is_valid_snapshot_profile_kind("scene"));
        assert!(!is_valid_snapshot_profile_kind("npc"));
    }

    #[test]
    fn canvas_cache_key() {
        assert_eq!(
            parse_canvas_cache_key("canvas_graph:1:node:2"),
            Some((1, 2))
        );
        assert_eq!(parse_canvas_cache_key("other"), None);
    }

    #[test]
    fn unique_ids() {
        assert!(validate_unique_ids([1, 2, 3], "id").is_ok());
        assert!(validate_unique_ids([1, 1], "id").is_err());
        assert!(validate_import_unique_ids([1, 2], "row.id").is_ok());
        let err = validate_import_unique_ids([1, 1], "row.id").unwrap_err();
        assert!(err.contains("导入包包含重复"));
        let err =
            validate_import_unique_key_strings(["a/1".into(), "a/1".into()].into_iter(), "ref")
                .unwrap_err();
        assert!(err.contains("导入包包含重复"));
    }

    #[test]
    fn hash_json_stable() {
        let v = json!({"a": 1});
        assert_eq!(hash_json(&v), hash_json(&v));
    }
}
