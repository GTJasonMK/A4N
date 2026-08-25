use super::identity::{
    GeneratedVideoProgressRow, VideoPromptProgressRow, VideoSlotKey, current_video_prompt_rows,
    latest_video_rows, video_slot_key_from_prompt, video_slot_key_from_video,
};
use crate::project::progress::ProjectProgressReadError;
use crate::project::progress::shared::ProgressCounts;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_domain::video::generated_video_prompt_id;
use std::collections::HashMap;

pub(super) struct VideoProgressFacts {
    pub(super) prompt_counts: ProgressCounts,
    pub(super) video_counts: ProgressCounts,
    pub(super) video_prompt_total: i64,
    pub(super) frame_prompt_total: i64,
    pub(super) segment_plan_total: i64,
}

pub(super) fn video_progress_facts_from_rows(
    prompts: &[VideoPromptProgressRow],
    videos: &[GeneratedVideoProgressRow],
) -> Result<VideoProgressFacts, ProjectProgressReadError> {
    let current_prompts = current_video_prompt_rows(prompts);
    let mut prompt_counts = ProgressCounts::default();
    let mut video_prompt_total = 0;
    let mut frame_prompt_total = 0;
    let mut segment_plan_total = 0;
    let mut prompt_slot_statuses = HashMap::new();
    let mut prompt_id_statuses = HashMap::new();
    for prompt in current_prompts {
        prompt_counts.add("chapter_video_prompt", prompt.id, &prompt.status)?;
        match prompt.prompt_type.as_str() {
            chapter_video_prompt::PROMPT_TYPE_VIDEO => {
                video_prompt_total += 1;
                prompt_id_statuses.insert(prompt.id, prompt.status.clone());
                if let Some(slot) = video_slot_key_from_prompt(prompt) {
                    prompt_slot_statuses.insert(slot, prompt.status.clone());
                }
            }
            chapter_video_prompt::PROMPT_TYPE_FRAME => frame_prompt_total += 1,
            chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN => segment_plan_total += 1,
            _ => {}
        }
    }

    let mut video_counts = ProgressCounts::default();
    for video in latest_video_rows(videos) {
        if should_count_video_for_progress(video, &prompt_slot_statuses, &prompt_id_statuses) {
            video_counts.add("generated_video", video.id, &video.status)?;
        }
    }

    Ok(VideoProgressFacts {
        prompt_counts,
        video_counts,
        video_prompt_total,
        frame_prompt_total,
        segment_plan_total,
    })
}

fn should_count_video_for_progress(
    video: &GeneratedVideoProgressRow,
    prompt_slot_statuses: &HashMap<VideoSlotKey, String>,
    prompt_id_statuses: &HashMap<i64, String>,
) -> bool {
    if video.status != generated_video::STATUS_FAILED {
        return true;
    }
    if let Some(slot) = video_slot_key_from_video(video)
        && let Some(status) = prompt_slot_statuses.get(&slot)
    {
        return status == chapter_video_prompt::STATUS_FAILED;
    }
    if let Some(prompt_id) = generated_video_prompt_id(video.metadata_json.as_ref())
        && let Some(status) = prompt_id_statuses.get(&prompt_id)
    {
        return status == chapter_video_prompt::STATUS_FAILED;
    }
    true
}
