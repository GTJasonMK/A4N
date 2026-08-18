//! 项目族/视频画布导入包纯校验与哈希工具（无 DB、无 Host）。

use std::collections::HashSet;
use std::hash::Hash;
use std::path::Path;

use a4n_core::models::media::media_asset::{self, FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectSourceMode;
use serde_json::Value;
use sha2::{Digest, Sha256};

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

use super::json_field_rules::required_json_i64_field;

/// 校验快照 payload 身份与哈希（调用方已解析 snapshot 根项目 id）。
pub fn validate_snapshot_payload_identity(
    snapshot_root_project_id: i64,
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
    Ok(())
}

/// freeform 视频项目结构约束。
pub fn validate_freeform_video_project(
    project: &novel_project::Model,
    label: &str,
) -> Result<(), String> {
    if project
        .source_mode()
        .map_err(|error| format!("{label}{error}"))?
        != ProjectSourceMode::Freeform
    {
        return Err(format!("{label}必须是 freeform video 项目"));
    }
    Ok(())
}

/// ID 列表唯一性（视频画布包文案）。
pub(crate) fn collect_unique_ids<I>(ids: I, label: &str) -> Result<HashSet<i64>, String>
where
    I: IntoIterator<Item = i64>,
{
    collect_unique_values(ids, |id| format!("视频画布项目包 ID 重复: {label}={id}"))
}

fn collect_unique_values<I, T, F>(values: I, duplicate_error: F) -> Result<HashSet<T>, String>
where
    I: IntoIterator<Item = T>,
    T: Eq + Hash,
    F: Fn(&T) -> String,
{
    let mut seen = HashSet::new();
    for value in values {
        if seen.contains(&value) {
            return Err(duplicate_error(&value));
        }
        seen.insert(value);
    }
    Ok(seen)
}

/// 导入包（family package）ID 唯一性。
pub fn validate_import_unique_ids<I>(ids: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = i64>,
{
    collect_unique_values(ids, |id| format!("导入包包含重复 {label}: {id}"))
        .map(|_| ())
}

/// 导入包复合 key 字符串唯一性。
pub fn validate_import_unique_key_strings<I>(keys: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    collect_unique_values(keys, |key| format!("导入包包含重复 {label}: {key}"))
        .map(|_| ())
}

/// 按字段取媒体资产路径。
pub fn media_asset_path<'a>(
    asset: &'a media_asset::Model,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match field {
        FILE_PATH_FIELD => Ok(asset.file_path.as_deref()),
        THUMBNAIL_PATH_FIELD => Ok(asset.thumbnail_path.as_deref()),
        _ => Err(format!("媒体文件字段无效: {field}")),
    }
}

/// 媒体包路径必填扩展名（小写）。
pub fn required_package_extension(path: &str) -> Result<String, String> {
    Ok(required_path_extension(path, "媒体包路径")?.to_ascii_lowercase())
}

/// 路径必填扩展名。
pub fn required_path_extension<'a>(path: &'a str, label: &str) -> Result<&'a str, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}缺少扩展名: {path}"))
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
    fn unique_ids() {
        assert!(validate_import_unique_ids([1, 2], "row.id").is_ok());
        let err = validate_import_unique_ids([1, 1], "row.id").unwrap_err();
        assert!(err.contains("导入包包含重复"));
        let err =
            validate_import_unique_key_strings(["a/1".into(), "a/1".into()], "ref").unwrap_err();
        assert!(err.contains("导入包包含重复"));
    }

    #[test]
    fn hash_json_stable() {
        let v = json!({"a": 1});
        assert_eq!(hash_json(&v), hash_json(&v));
    }
}
