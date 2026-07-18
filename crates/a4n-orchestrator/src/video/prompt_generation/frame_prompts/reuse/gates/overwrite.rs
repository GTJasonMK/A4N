use super::*;

pub(in crate::video::prompt_generation::frame_prompts) fn ensure_video_prompts_can_regenerate(
    rows: &[chapter_video_prompt::Model],
    label: &str,
) -> Result<(), AppError> {
    if let Some(row) = rows.iter().find(|row| row.generated_video_id.is_some()) {
        return Err(AppError::InvalidArgument(format!(
            "已存在由{label} #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
            row.shot_number
        )));
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) fn ensure_chat_video_prompts_can_generate_without_overwriting(
    all_existing_rows: &[chapter_video_prompt::Model],
    matching_rows: &[chapter_video_prompt::Model],
    reusable_prompts: &BTreeMap<i32, ChatVideoPromptShot>,
    segment_plan: &[VideoSegmentPlan],
    regenerate_existing: bool,
    generation_limit: usize,
) -> Result<(), AppError> {
    let mut allowed_reused_generated_row_ids = HashSet::new();
    let mut seen_matching_shots = HashSet::new();
    for row in matching_rows {
        if !seen_matching_shots.insert(row.shot_number) {
            continue;
        }
        let Some(reusable) = reusable_prompts.get(&row.shot_number) else {
            continue;
        };
        if !regenerate_existing
            && row.generated_video_id.is_some()
            && !reusable.chat_prompt.is_empty()
        {
            allowed_reused_generated_row_ids.insert(row.id);
        }
    }
    let existing_by_shot = all_existing_rows.iter().fold(
        HashMap::<i32, Vec<&chapter_video_prompt::Model>>::new(),
        |mut rows, row| {
            rows.entry(row.shot_number).or_default().push(row);
            rows
        },
    );
    let mut generated_this_run = 0usize;
    for segment in segment_plan {
        let shot_number = segment.segment_number;
        let reusable = reusable_prompts.get(&shot_number);
        if reusable.is_some() && (!regenerate_existing || generated_this_run >= generation_limit) {
            if let Some(rows) = existing_by_shot.get(&shot_number)
                && let Some(row) = rows.iter().find(|row| {
                    row.generated_video_id.is_some()
                        && !allowed_reused_generated_row_ids.contains(&row.id)
                })
            {
                return Err(AppError::InvalidArgument(format!(
                    "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                    row.shot_number
                )));
            }
            continue;
        } else if reusable.is_some()
            && regenerate_existing
            && generated_this_run >= generation_limit
        {
            continue;
        }

        if generated_this_run >= generation_limit {
            continue;
        }
        if let Some(rows) = existing_by_shot.get(&shot_number)
            && let Some(row) = rows.iter().find(|row| row.generated_video_id.is_some())
        {
            return Err(AppError::InvalidArgument(format!(
                "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                row.shot_number
            )));
        }
        generated_this_run += 1;
    }
    Ok(())
}
