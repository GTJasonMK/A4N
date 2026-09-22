use serde_json::{Map, Value};

pub(super) fn required_snapshot_string(
    snapshot: &Map<String, Value>,
    field: &str,
    video_id: i64,
) -> Result<String, String> {
    let value = optional_snapshot_string(snapshot, field, video_id)?
        .ok_or_else(|| format!("视频 #{video_id} provider 请求快照缺少 {field}，无法直接预览"))?;
    if value.trim().is_empty() {
        return Err(format!(
            "视频 #{video_id} provider 请求快照字段 {field} 不能为空"
        ));
    }
    Ok(value)
}

pub(super) fn optional_snapshot_string(
    snapshot: &Map<String, Value>,
    field: &str,
    video_id: i64,
) -> Result<Option<String>, String> {
    let Some(value) = snapshot.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| format!("视频 #{video_id} provider 请求快照字段 {field} 必须是字符串"))
}
