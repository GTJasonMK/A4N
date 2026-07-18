use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::actions::video_generation) fn boundary_frame_numbers_for_prompt(
    prompt: &chapter_video_prompt::Model,
) -> Result<(i32, i32), String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成视频提示词后再生成视频",
            prompt.id
        )
    })?;
    let video_unit = metadata
        .get("video_unit")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} 缺少 metadata_json.video_unit，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    let first = required_video_unit_frame_number(
        prompt,
        video_unit,
        "start_frame_number",
        "起始边界帧编号",
    )?;
    let last =
        required_video_unit_frame_number(prompt, video_unit, "end_frame_number", "结束边界帧编号")?;
    Ok((first, last))
}

fn required_video_unit_frame_number(
    prompt: &chapter_video_prompt::Model,
    video_unit: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    label: &str,
) -> Result<i32, String> {
    let value = video_unit
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 缺少有效{label}，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    i32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 必须是正整数，实际为 {value}",
                prompt.id
            )
        })
}
