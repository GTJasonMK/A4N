//! 成片后处理（assembly_postprocess_edit）纯规则。

use a4n_domain::AssemblyPostprocessOperation;
use serde_json::{Map, Value};

/// 后处理编辑 metadata 中的版本链字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostprocessEditMetadata {
    pub push_video_id: i64,
    pub base_video_id: i64,
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
    let operation = metadata
        .get("operation")
        .ok_or_else(|| format!("后处理编辑视频 #{video_id} 缺少 postprocess_edit.operation"))?;
    serde_json::from_value::<AssemblyPostprocessOperation>(operation.clone()).map_err(|_| {
        format!("后处理编辑视频 #{video_id} 的 postprocess_edit.operation 无效")
    })?;
    let _source_video_id =
        optional_positive_i64(metadata.get("source_video_id")).map_err(|err| {
            format!("后处理编辑视频 #{video_id} 的 postprocess_edit.source_video_id {err}")
        })?;
    Ok(PostprocessEditMetadata {
        push_video_id,
        base_video_id,
    })
}

/// 在已知 base/source 时长下校验并计算后处理时间与结果时长（无 IO）。
pub fn postprocess_edit_timing(
    base_duration_secs: f32,
    operation: AssemblyPostprocessOperation,
    at_secs: Option<f32>,
    start_secs: Option<f32>,
    end_secs: Option<f32>,
    source_duration_secs: Option<f32>,
) -> Result<(f32, f32, f32), String> {
    if !base_duration_secs.is_finite() || base_duration_secs <= 0.0 {
        return Err("当前片段时长必须是大于 0 的有限数".to_string());
    }
    match operation {
        AssemblyPostprocessOperation::Insert => {
            if start_secs.is_some() || end_secs.is_some() {
                return Err("插入操作不接受删除/替换时间范围".to_string());
            }
            let at = at_secs.ok_or_else(|| "插入操作缺少插入时间".to_string())?;
            if !at.is_finite() || at < 0.0 || at > base_duration_secs {
                return Err("插入时间超出当前片段范围".to_string());
            }
            let source_duration = required_source_duration(source_duration_secs)?;
            let output_duration = required_output_duration(base_duration_secs + source_duration)?;
            Ok((at, at, output_duration))
        }
        AssemblyPostprocessOperation::Delete => {
            if source_duration_secs.is_some() {
                return Err("删除操作不接受来源视频时长".to_string());
            }
            let (start, end) = required_edit_range(
                base_duration_secs,
                at_secs,
                start_secs,
                end_secs,
            )?;
            let output_duration =
                required_output_duration(base_duration_secs - (end - start))?;
            Ok((start, end, output_duration))
        }
        AssemblyPostprocessOperation::Replace => {
            let (start, end) = required_edit_range(
                base_duration_secs,
                at_secs,
                start_secs,
                end_secs,
            )?;
            let source_duration = required_source_duration(source_duration_secs)?;
            let output_duration = required_output_duration(
                base_duration_secs - (end - start) + source_duration,
            )?;
            Ok((start, end, output_duration))
        }
    }
}

fn required_edit_range(
    base_duration_secs: f32,
    at_secs: Option<f32>,
    start_secs: Option<f32>,
    end_secs: Option<f32>,
) -> Result<(f32, f32), String> {
    if at_secs.is_some() {
        return Err("删除/替换操作不接受插入时间".to_string());
    }
    let start = start_secs.ok_or_else(|| "删除/替换操作缺少起始时间".to_string())?;
    let end = end_secs.ok_or_else(|| "删除/替换操作缺少结束时间".to_string())?;
    if !start.is_finite()
        || !end.is_finite()
        || start < 0.0
        || end <= start
        || end > base_duration_secs
    {
        return Err("删除/替换时间范围无效".to_string());
    }
    Ok((start, end))
}

fn required_source_duration(value: Option<f32>) -> Result<f32, String> {
    value
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .ok_or_else(|| "插入或替换来源视频时长无效".to_string())
}

fn required_output_duration(value: f32) -> Result<f32, String> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("后处理结果时长必须是大于 0 的有限数".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    fn postprocess_edit_timing_validates_insert_and_delete() {
        let (a, b, duration) = postprocess_edit_timing(
            8.0,
            AssemblyPostprocessOperation::Insert,
            Some(2.0),
            None,
            None,
            Some(1.5),
        )
        .unwrap();
        assert_eq!((a, b, duration), (2.0, 2.0, 9.5));

        let (s, e, duration) = postprocess_edit_timing(
            8.0,
            AssemblyPostprocessOperation::Delete,
            None,
            Some(1.0),
            Some(3.0),
            None,
        )
        .unwrap();
        assert_eq!((s, e, duration), (1.0, 3.0, 6.0));

        assert!(
            postprocess_edit_timing(
                8.0,
                AssemblyPostprocessOperation::Insert,
                Some(9.0),
                None,
                None,
                Some(1.0),
            )
            .is_err()
        );
        assert!(
            postprocess_edit_timing(
                8.0,
                AssemblyPostprocessOperation::Delete,
                None,
                Some(0.0),
                Some(8.0),
                None,
            )
            .is_err()
        );
    }
}
