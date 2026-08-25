use a4n_domain::video::generated_video_prompt_id;
use chrono::NaiveDateTime;
use sea_orm::FromQueryResult;
use std::collections::HashMap;

#[derive(Debug, FromQueryResult)]
pub(super) struct VideoPromptProgressRow {
    pub(super) id: i64,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: String,
    pub(super) prompt_type: String,
    pub(super) shot_number: i32,
    pub(super) status: String,
    pub(super) updated_at: NaiveDateTime,
}

#[derive(Debug, FromQueryResult)]
pub(super) struct GeneratedVideoProgressRow {
    pub(super) id: i64,
    pub(super) project_id: i64,
    pub(super) chapter_id: Option<i64>,
    pub(super) workflow_mode: Option<String>,
    pub(super) status: String,
    pub(super) metadata_json: Option<serde_json::Value>,
    pub(super) updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VideoPromptProgressKey {
    chapter_id: i64,
    workflow_mode: String,
    prompt_type: String,
    shot_number: i32,
}

impl From<&VideoPromptProgressRow> for VideoPromptProgressKey {
    fn from(prompt: &VideoPromptProgressRow) -> Self {
        Self {
            chapter_id: prompt.chapter_id,
            workflow_mode: prompt.workflow_mode.clone(),
            prompt_type: prompt.prompt_type.clone(),
            shot_number: prompt.shot_number,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct VideoSlotKey {
    chapter_id: i64,
    workflow_mode: String,
    shot_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VideoProgressVideoKey {
    Slot(VideoSlotKey),
    Prompt(i64),
    Record(i64),
}

pub(super) fn current_video_prompt_rows(
    prompts: &[VideoPromptProgressRow],
) -> Vec<&VideoPromptProgressRow> {
    let mut by_key = HashMap::<VideoPromptProgressKey, &VideoPromptProgressRow>::new();
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

pub(super) fn latest_video_rows(
    videos: &[GeneratedVideoProgressRow],
) -> Vec<&GeneratedVideoProgressRow> {
    let mut by_key = HashMap::<VideoProgressVideoKey, &GeneratedVideoProgressRow>::new();
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
    candidate: &VideoPromptProgressRow,
    existing: &VideoPromptProgressRow,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_row_is_newer(
    candidate: &GeneratedVideoProgressRow,
    existing: &GeneratedVideoProgressRow,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_progress_video_key(video: &GeneratedVideoProgressRow) -> VideoProgressVideoKey {
    if let Some(slot) = video_slot_key_from_video(video) {
        return VideoProgressVideoKey::Slot(slot);
    }
    if let Some(prompt_id) = generated_video_prompt_id(video.metadata_json.as_ref()) {
        return VideoProgressVideoKey::Prompt(prompt_id);
    }
    VideoProgressVideoKey::Record(video.id)
}

pub(super) fn video_slot_key_from_prompt(prompt: &VideoPromptProgressRow) -> Option<VideoSlotKey> {
    (prompt.shot_number > 0).then(|| VideoSlotKey {
        chapter_id: prompt.chapter_id,
        workflow_mode: prompt.workflow_mode.clone(),
        shot_number: prompt.shot_number,
    })
}

pub(super) fn video_slot_key_from_video(video: &GeneratedVideoProgressRow) -> Option<VideoSlotKey> {
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

fn video_metadata_i64(video: &GeneratedVideoProgressRow, key: &str) -> Option<i64> {
    video.metadata_json.as_ref()?.get(key)?.as_i64()
}
