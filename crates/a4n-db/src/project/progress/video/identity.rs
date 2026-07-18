use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VideoPromptProgressKey {
    chapter_id: i64,
    workflow_mode: String,
    prompt_type: String,
    shot_number: i32,
}

impl From<&chapter_video_prompt::Model> for VideoPromptProgressKey {
    fn from(prompt: &chapter_video_prompt::Model) -> Self {
        Self {
            chapter_id: prompt.chapter_id,
            workflow_mode: prompt.workflow_mode.clone(),
            prompt_type: prompt.prompt_type.clone(),
            shot_number: prompt.shot_number,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VideoSlotKey {
    chapter_id: i64,
    workflow_mode: String,
    shot_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum VideoProgressVideoKey {
    Slot(VideoSlotKey),
    Prompt(i64),
    Record(i64),
}

pub(super) fn current_video_prompt_rows(
    prompts: &[chapter_video_prompt::Model],
) -> Vec<&chapter_video_prompt::Model> {
    let mut by_key = BTreeMap::<VideoPromptProgressKey, &chapter_video_prompt::Model>::new();
    for prompt in prompts {
        let key = VideoPromptProgressKey::from(prompt);
        match by_key.get(&key) {
            Some(existing) if !prompt_row_is_newer(prompt, existing) => {}
            _ => {
                by_key.insert(key, prompt);
            }
        }
    }
    by_key.into_values().collect()
}

pub(super) fn latest_video_rows(videos: &[generated_video::Model]) -> Vec<&generated_video::Model> {
    let mut by_key = BTreeMap::<VideoProgressVideoKey, &generated_video::Model>::new();
    for video in videos {
        let key = video_progress_video_key(video);
        match by_key.get(&key) {
            Some(existing) if !video_row_is_newer(video, existing) => {}
            _ => {
                by_key.insert(key, video);
            }
        }
    }
    by_key.into_values().collect()
}

fn prompt_row_is_newer(
    candidate: &chapter_video_prompt::Model,
    existing: &chapter_video_prompt::Model,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_row_is_newer(
    candidate: &generated_video::Model,
    existing: &generated_video::Model,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_progress_video_key(video: &generated_video::Model) -> VideoProgressVideoKey {
    if let Some(slot) = video_slot_key_from_video(video) {
        return VideoProgressVideoKey::Slot(slot);
    }
    if let Some(prompt_id) = video_metadata_i64(video, "prompt_id") {
        return VideoProgressVideoKey::Prompt(prompt_id);
    }
    VideoProgressVideoKey::Record(video.id)
}

pub(super) fn video_slot_key_from_prompt(
    prompt: &chapter_video_prompt::Model,
) -> Option<VideoSlotKey> {
    (prompt.shot_number > 0).then(|| VideoSlotKey {
        chapter_id: prompt.chapter_id,
        workflow_mode: prompt.workflow_mode.clone(),
        shot_number: prompt.shot_number,
    })
}

pub(super) fn video_slot_key_from_video(video: &generated_video::Model) -> Option<VideoSlotKey> {
    Some(VideoSlotKey {
        chapter_id: video.chapter_id?,
        workflow_mode: video.workflow_mode.clone().unwrap_or_default(),
        shot_number: video_metadata_i64(video, "shot_number").and_then(|value| {
            i32::try_from(value)
                .ok()
                .and_then(|value| (value > 0).then_some(value))
        })?,
    })
}

pub(super) fn video_metadata_i64(video: &generated_video::Model, key: &str) -> Option<i64> {
    video.metadata_json.as_ref()?.get(key)?.as_i64()
}
