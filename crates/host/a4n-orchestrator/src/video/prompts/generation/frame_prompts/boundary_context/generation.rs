use super::super::*;

fn frame_context_value(frame: &VideoFramePrompt) -> Value {
    json!({
        "frame_number": frame.frame_number,
        "title": frame.title.as_str(),
        "scene_summary": frame.scene_summary.as_str(),
        "frame_prompt": frame.frame_prompt.as_str(),
        "blocking_state": frame.blocking_state.clone(),
        "lighting_state": frame.lighting_state.clone(),
        "segment_numbers": &frame.segment_numbers
    })
}
pub(in crate::video) fn boundary_frame_generation_context(
    segment_plan: &[VideoSegmentPlan],
    completed_frames: &[VideoFramePrompt],
    frame_number: i32,
    previous_frame_image: Option<Value>,
) -> Result<Value, AppError> {
    let expected_segment_numbers =
        boundary_frame_expected_segment_numbers(segment_plan, frame_number)?;
    let required_duration_secs =
        boundary_frame_expected_duration(segment_plan, &expected_segment_numbers)?;
    let units = video_units_for_segment_plan(segment_plan, required_duration_secs)?;
    let previous_unit = frame_number
        .checked_sub(2)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| units.get(index));
    let current_unit = usize::try_from(frame_number - 1)
        .ok()
        .and_then(|index| units.get(index));
    let next_unit = usize::try_from(frame_number)
        .ok()
        .and_then(|index| units.get(index));
    let previous_segment = previous_unit.map(|unit| &unit.prompt_segment);
    let current_segment = current_unit
        .or(previous_unit)
        .map(|unit| &unit.prompt_segment)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("边界帧 #{} 缺少对应视频单元", frame_number))
        })?;
    let next_segment = next_unit.map(|unit| &unit.prompt_segment);
    let previous_frame = completed_frames.last().map(frame_context_value);
    let role = boundary_frame_role(segment_plan, frame_number)?;
    let required_aspect_ratio = current_segment.aspect_ratio.as_str();
    Ok(json!({
        "frame_number": frame_number,
        "total_frame_count": expected_boundary_frame_count(segment_plan),
        "role": role,
        "expected_segment_numbers": expected_segment_numbers,
        "required_duration_secs": required_duration_secs,
        "required_aspect_ratio": required_aspect_ratio,
        "previous_frame": previous_frame,
        "previous_frame_image": previous_frame_image,
        "reference_slot_instruction": if frame_number == 1 {
            "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不得输出 reference_image_ids，也不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。"
        } else {
            "如果 previous_frame_image 存在，后端会把上一帧实际图片绑定为参考图 1；其他素材按 constraints.reference_slots 后续顺序称为参考图 2、参考图 3。不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。"
        },
        "previous_segment": previous_segment,
        "current_segment": current_segment,
        "next_segment": next_segment,
        "continuity_instruction": if role == "chain_start" {
            "这是整条视频执行链的首个边界帧，不要编造上一帧；必须建立人物、场景锚点、构图、道具和光源基准。"
        } else if role == "shared_boundary" {
            "这是相邻视频单元共享的同一张边界帧：它既是前一单元稳定尾态，也是后一单元真实起点。必须只保留双方合同都允许的站位、道具、视线、光线和镜头方向；不得额外生成一个独立过渡画面。"
        } else {
            "这是整条视频执行链的最终边界帧；必须稳定留下最后单元结束时的人物姿态、空间锚点、道具状态、情绪和镜头方向。"
        }
    }))
}
