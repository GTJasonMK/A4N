use serde_json::{Value, json};

use crate::video::VideoDbError;

const SYSTEM_VIDEO_METADATA_KEYS: &[&str] = &[
    "boundary_frame_image_count",
    "boundary_frame_image_ids",
    "boundary_frame_image_url_count",
    "boundary_frame_request",
    "canvas",
    "config_id",
    "first_frame_image_id",
    "material_reference_image_count",
    "material_reference_image_ids",
    "prompt_id",
    "provider_backend",
    "provider_reference_image_count",
    "provider_reference_image_ids",
    "provider_request_snapshot",
    "prompt_reference_image_count",
    "prompt_reference_image_ids",
    "provider_response",
    "reference_image_count",
    "reference_image_ids",
    "reference_media",
    "runtime_config_snapshot",
    "source",
    "source_prompt_type",
    "start_frame_image_id",
];

pub fn merge_video_provider_response_metadata(
    existing: Option<Value>,
    incoming: Option<Value>,
) -> Result<Option<Value>, VideoDbError> {
    let incoming = incoming.filter(|value| !value.is_null());
    match (existing, incoming) {
        (Some(Value::Object(mut base)), Some(provider)) => {
            base.insert("provider_response".into(), provider);
            Ok(Some(Value::Object(base)))
        }
        (None, Some(provider)) => Ok(Some(json!({ "provider_response": provider }))),
        (existing, None) => Ok(existing),
        (Some(_), Some(_)) => Err(VideoDbError::InvalidArgument(
            "视频 metadata 必须是对象，不能写入 provider_response".to_string(),
        )),
    }
}

pub fn validate_video_metadata_update_preserves_system_fields(
    existing: Option<&Value>,
    incoming: Option<&Value>,
    clear_metadata_json: bool,
) -> Result<(), VideoDbError> {
    let Some(existing) = existing.and_then(Value::as_object) else {
        return Ok(());
    };
    let protected = SYSTEM_VIDEO_METADATA_KEYS
        .iter()
        .filter_map(|key| existing.get(*key).map(|value| (*key, value)))
        .collect::<Vec<_>>();
    if protected.is_empty() {
        return Ok(());
    }

    if clear_metadata_json {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 metadata 包含系统字段 {}，不能清空",
            protected_video_metadata_keys(&protected)
        )));
    }
    let Some(incoming) = incoming else {
        return Ok(());
    };
    let incoming = incoming.as_object().ok_or_else(|| {
        VideoDbError::InvalidArgument(format!(
            "视频 metadata 更新必须是对象，并保留系统字段 {}",
            protected_video_metadata_keys(&protected)
        ))
    })?;
    for (key, existing_value) in protected {
        if incoming.get(key) != Some(existing_value) {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频 metadata 系统字段 {key} 不能修改或删除"
            )));
        }
    }
    Ok(())
}

fn protected_video_metadata_keys(protected: &[(&str, &Value)]) -> String {
    protected
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>()
        .join(", ")
}
