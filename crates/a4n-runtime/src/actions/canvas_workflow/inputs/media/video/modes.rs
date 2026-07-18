use super::super::super::contracts::GenerateVideoInputMode;
use serde_json::Value;

pub(super) fn generate_video_input_mode(
    value: Option<&Value>,
) -> Result<GenerateVideoInputMode, String> {
    match value.and_then(Value::as_str).map(str::trim) {
        Some("text_only") => Ok(GenerateVideoInputMode::TextOnly),
        Some("first_frame") => Ok(GenerateVideoInputMode::FirstFrame),
        Some("first_end_frame") => Ok(GenerateVideoInputMode::FirstEndFrame),
        Some("reference_images") => Ok(GenerateVideoInputMode::ReferenceImages),
        Some(other) if !other.is_empty() => {
            Err(format!("GenerateVideoNode input_mode 无效: {other}"))
        }
        _ => Ok(GenerateVideoInputMode::TextOnly),
    }
}

pub(super) fn validate_generate_video_image_inputs(
    mode: GenerateVideoInputMode,
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: &[i64],
) -> Result<(), String> {
    match mode {
        GenerateVideoInputMode::TextOnly => {
            if first_frame_image_id.is_some()
                || last_frame_image_id.is_some()
                || !reference_image_ids.is_empty()
            {
                return Err("GenerateVideoNode 当前为仅提示词模式，不能连接图片输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstFrame => {
            if first_frame_image_id.is_none() {
                return Err("GenerateVideoNode 首帧模式缺少 first_frame 输入".to_string());
            }
            if last_frame_image_id.is_some() || !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首帧模式只能连接 first_frame 输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstEndFrame => {
            if first_frame_image_id.is_none() || last_frame_image_id.is_none() {
                return Err(
                    "GenerateVideoNode 首尾帧模式必须同时连接 first_frame 和 end_frame 输入"
                        .to_string(),
                );
            }
            if !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首尾帧模式不能连接 reference_image 输入".to_string());
            }
        }
        GenerateVideoInputMode::ReferenceImages => {
            if reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 参考图模式缺少 reference_image 输入".to_string());
            }
            if first_frame_image_id.is_some() || last_frame_image_id.is_some() {
                return Err("GenerateVideoNode 参考图模式只能连接 reference_image 输入".to_string());
            }
        }
    }
    Ok(())
}
