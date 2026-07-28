//! 成片后处理（assembly_postprocess_edit）纯规则。

use serde_json::{Map, Value};

pub const ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE: &str = "assembly_postprocess_edit";

/// 后处理编辑 metadata 中的版本链字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostprocessEditMetadata {
    pub push_video_id: i64,
    pub base_video_id: i64,
}

/// 校验后处理操作名。
pub fn validate_postprocess_operation(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "insert" => Ok("insert"),
        "delete" => Ok("delete"),
        "replace" => Ok("replace"),
        "" => Err("请选择后处理操作".to_string()),
        other => Err(format!("后处理操作无效：{other}")),
    }
}

fn required_positive_i64(value: Option<&Value>) -> Result<i64, String> {
    value
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "必须是正整数".to_string())
}

fn optional_positive_i64(value: Option<&Value>) -> Result<Option<i64>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|id| *id > 0)
            .map(Some)
            .ok_or_else(|| "必须为空或正整数".to_string()),
    }
}

/// 从 metadata_json 解析 postprocess_edit 版本链字段。
pub fn parse_postprocess_edit_metadata(
    video_id: i64,
    metadata_json: Option<&Value>,
) -> Result<PostprocessEditMetadata, String> {
    let metadata = metadata_json
        .and_then(|metadata| metadata.get("postprocess_edit"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("后处理编辑视频 #{video_id} 缺少 postprocess_edit metadata"))?;
    parse_postprocess_edit_object(video_id, metadata)
}

fn parse_postprocess_edit_object(
    video_id: i64,
    metadata: &Map<String, Value>,
) -> Result<PostprocessEditMetadata, String> {
    let push_video_id = required_positive_i64(metadata.get("push_video_id")).map_err(|err| {
        format!("后处理编辑视频 #{video_id} 的 postprocess_edit.push_video_id {err}")
    })?;
    let base_video_id = required_positive_i64(metadata.get("base_video_id")).map_err(|err| {
        format!("后处理编辑视频 #{video_id} 的 postprocess_edit.base_video_id {err}")
    })?;
    let _operation = metadata
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("后处理编辑视频 #{video_id} 缺少 postprocess_edit.operation"))
        .and_then(validate_postprocess_operation)?;
    let _source_video_id =
        optional_positive_i64(metadata.get("source_video_id")).map_err(|err| {
            format!("后处理编辑视频 #{video_id} 的 postprocess_edit.source_video_id {err}")
        })?;
    Ok(PostprocessEditMetadata {
        push_video_id,
        base_video_id,
    })
}

/// 在已知 base/source 时长下计算后处理时间范围（无 IO）。
pub fn postprocess_edit_range_bounds(
    base_duration_secs: f32,
    operation: &str,
    at_secs: Option<f32>,
    start_secs: Option<f32>,
    end_secs: Option<f32>,
    source_duration_secs: Option<f32>,
) -> Result<(f32, f32, Option<f32>), String> {
    let base_duration = base_duration_secs.max(0.1);
    match operation {
        "insert" => {
            let at = at_secs.ok_or_else(|| "插入操作缺少插入时间".to_string())?;
            if !at.is_finite() || at < 0.0 || at > base_duration {
                return Err("插入时间超出当前片段范围".to_string());
            }
            Ok((at, at, source_duration_secs))
        }
        "delete" | "replace" => {
            let start = start_secs.ok_or_else(|| "删除/替换操作缺少起始时间".to_string())?;
            let end = end_secs.ok_or_else(|| "删除/替换操作缺少结束时间".to_string())?;
            if !start.is_finite()
                || !end.is_finite()
                || start < 0.0
                || end <= start
                || end > base_duration + 0.05
            {
                return Err("删除/替换时间范围无效".to_string());
            }
            Ok((start, end.min(base_duration), source_duration_secs))
        }
        _ => Err(format!("后处理操作无效：{operation}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_postprocess_operation_accepts_only_supported() {
        assert_eq!(validate_postprocess_operation("insert"), Ok("insert"));
        assert_eq!(validate_postprocess_operation(" delete "), Ok("delete"));
        assert!(validate_postprocess_operation("").is_err());
        assert!(validate_postprocess_operation("crop").is_err());
    }

    #[test]
    fn parse_postprocess_edit_metadata_requires_version_chain() {
        let ok = parse_postprocess_edit_metadata(
            9,
            Some(&json!({
                "postprocess_edit": {
                    "push_video_id": 1,
                    "base_video_id": 2,
                    "operation": "insert"
                }
            })),
        )
        .unwrap();
        assert_eq!(ok.push_video_id, 1);
        assert_eq!(ok.base_video_id, 2);

        let err = parse_postprocess_edit_metadata(9, Some(&json!({}))).unwrap_err();
        assert!(err.contains("缺少 postprocess_edit metadata"));
    }

    #[test]
    fn postprocess_edit_range_bounds_validates_insert_and_delete() {
        let (a, b, src) =
            postprocess_edit_range_bounds(8.0, "insert", Some(2.0), None, None, Some(1.5)).unwrap();
        assert_eq!((a, b, src), (2.0, 2.0, Some(1.5)));

        let (s, e, _) =
            postprocess_edit_range_bounds(8.0, "delete", None, Some(1.0), Some(3.0), None).unwrap();
        assert_eq!((s, e), (1.0, 3.0));

        assert!(postprocess_edit_range_bounds(8.0, "insert", Some(9.0), None, None, None).is_err());
        assert!(
            postprocess_edit_range_bounds(8.0, "delete", None, Some(3.0), Some(1.0), None).is_err()
        );
    }
}
