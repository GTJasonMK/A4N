use super::*;

pub(in crate::video::prompt_json) fn derive_chat_video(segment: &VideoSegmentPlan) -> Value {
    let input_images = segment
        .reference_image_ids
        .iter()
        .enumerate()
        .map(|(index, image_id)| {
            json!({
                "image_id": image_id,
                "upload_order": index + 1,
                "role": "reference",
                "usage": "用于当前片段的人物、场景、道具或风格一致性约束"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "source_span_ids": segment.source_span_ids,
        "timeline": [{
            "time": format!("0-{}秒", segment.duration_secs),
            "action": performance_summary(segment)
        }],
        "continuity": segment_end_summary(segment),
        "input_images": input_images
    })
}
