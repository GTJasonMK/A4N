use super::super::types::VideoStudioProjectImportError;
use serde_json::Value;
use std::collections::HashMap;

pub(in crate::canvas::transfer) fn remap_video_studio_json_i64_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), VideoStudioProjectImportError> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let old_id = value.as_i64().ok_or_else(|| {
        VideoStudioProjectImportError::InvalidPackage(format!(
            "视频画布项目包字段 {key} 必须是整数 ID"
        ))
    })?;
    let new_id = remap_video_studio_required_id(map, old_id, key)?;
    object.insert(key.to_string(), Value::Number(new_id.into()));
    Ok(())
}

pub(in crate::canvas::transfer) fn remap_video_studio_json_i64_array_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), VideoStudioProjectImportError> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let values = value.as_array().ok_or_else(|| {
        VideoStudioProjectImportError::InvalidPackage(format!(
            "视频画布项目包字段 {key} 必须是整数 ID 数组"
        ))
    })?;
    let remapped = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let old_id = value.as_i64().ok_or_else(|| {
                VideoStudioProjectImportError::InvalidPackage(format!(
                    "视频画布项目包字段 {key}[{index}] 必须是整数 ID"
                ))
            })?;
            let new_id = remap_video_studio_required_id(map, old_id, key)?;
            Ok(Value::Number(new_id.into()))
        })
        .collect::<Result<Vec<_>, VideoStudioProjectImportError>>()?;
    object.insert(key.to_string(), Value::Array(remapped));
    Ok(())
}

pub(in crate::canvas::transfer) fn remap_video_studio_required_id(
    map: &HashMap<i64, i64>,
    old_id: i64,
    field: &str,
) -> Result<i64, VideoStudioProjectImportError> {
    map.get(&old_id).copied().ok_or_else(|| {
        VideoStudioProjectImportError::InvalidPackage(format!(
            "导入视频画布项目时无法重映射 {field}: {old_id}"
        ))
    })
}

pub(in crate::canvas::transfer) fn remap_video_studio_optional_id(
    map: &HashMap<i64, i64>,
    old_id: Option<i64>,
    field: &str,
) -> Result<Option<i64>, VideoStudioProjectImportError> {
    old_id
        .map(|id| remap_video_studio_required_id(map, id, field))
        .transpose()
}
