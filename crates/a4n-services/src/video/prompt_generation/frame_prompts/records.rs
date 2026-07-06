use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::super::super::library::ensure_video_reference_ids_known;
use super::super::super::prompt_json::{
    ChatVideoPromptShot, StoryboardPanel, VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
    validate_chat_video_prompt_shot_against_segment, validate_reference_label_text,
    validate_video_prompt_shot_against_segment,
};
use super::super::super::prompt_records::reusable_rows_are_complete;
use super::super::super::prompt_store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};
use super::super::super::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT,
    FRAME_SEGMENT_PLAN_CONTRACT_VERSION, PromptOutputLanguage, STORYBOARD_PROMPT_CONTRACT_VERSION,
    STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};
use super::{
    VideoUnitPlan, boundary_frame_expected_segment_numbers, expected_boundary_frame_count,
    expected_video_unit_count, storyboard_expected_segment_numbers,
};
use crate::prompt_language::with_prompt_output_language_metadata;

pub(super) fn complete_reusable_rows(
    rows: Vec<chapter_video_prompt::Model>,
    expected_count: usize,
) -> Option<Vec<chapter_video_prompt::Model>> {
    reusable_rows_are_complete(&rows, expected_count).then_some(rows)
}

pub(in crate::video) fn initial_first_frame_rows(
    rows: Vec<chapter_video_prompt::Model>,
) -> Vec<chapter_video_prompt::Model> {
    rows.into_iter()
        .filter(|row| row.shot_number == 1)
        .collect()
}

pub(in crate::video) fn frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "boundary_frame", "边界帧提示词")
}

pub(in crate::video) fn first_frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "first_frame", "首帧提示词")
}

pub(in crate::video) fn storyboard_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "storyboard_frame", "分镜提示词")
}

fn saved_frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
    metadata_key: &str,
    label: &str,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    rows.iter()
        .map(|row| {
            let value = row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get(metadata_key))
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存{label} #{} 缺少 {metadata_key} 元数据",
                        row.shot_number,
                    ))
                })?;
            serde_json::from_value::<VideoFramePrompt>(value).map_err(|err| {
                AppError::InvalidArgument(format!(
                    "已保存{label} #{} 的 {metadata_key} 元数据无效：{}",
                    row.shot_number, err,
                ))
            })
        })
        .collect()
}

pub(in crate::video) fn validate_frame_prompts(
    frame_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    let expected_frame_count = expected_boundary_frame_count(segment_plan);
    if frame_prompts.len() != expected_frame_count {
        return Err(AppError::InvalidArgument(format!(
            "视频首尾帧提示词数量必须等于片段数 * 2：当前 {} 条，片段数 {}，应为 {} 条",
            frame_prompts.len(),
            segment_plan.len(),
            expected_frame_count
        )));
    }
    for (idx, frame) in frame_prompts.iter().enumerate() {
        let expected_frame_number = (idx + 1) as i32;
        if frame.frame_number != expected_frame_number {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number 必须从 1 连续编号，位置 {} 应为 {}，实际为 {}",
                idx + 1,
                expected_frame_number,
                frame.frame_number
            )));
        }
        let expected_segment_numbers =
            boundary_frame_expected_segment_numbers(segment_plan, frame.frame_number)?;
        if frame.segment_numbers != expected_segment_numbers {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number {} segment_numbers 必须为 {:?}，实际为 {:?}",
                frame.frame_number, expected_segment_numbers, frame.segment_numbers
            )));
        }
        if frame.duration_secs < 1 {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number {} 时长必须大于 0，实际为 {}s",
                frame.frame_number, frame.duration_secs
            )));
        }
        for segment_number in &frame.segment_numbers {
            if !segment_plan
                .iter()
                .any(|segment| segment.segment_number == *segment_number)
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频首尾帧提示词 frame_number {} 引用了不存在的片段 {}",
                    frame.frame_number, segment_number
                )));
            }
        }
        ensure_video_reference_ids_known(
            &frame.reference_image_ids,
            allowed_reference_ids,
            &format!("视频边界帧 #{}", frame.frame_number),
        )?;
        validate_reference_label_text(
            &frame.frame_prompt,
            &format!("视频边界帧 #{} frame_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_reference_label_text(
            &frame.negative_prompt,
            &format!("视频边界帧 #{} negative_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        ensure_no_storyboard_panels(frame, &format!("视频边界帧 #{}", frame.frame_number))?;
    }
    Ok(())
}

pub(in crate::video) fn validate_first_frame_prompts(
    first_frame_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    if first_frame_prompts.len() != FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "首帧串联模式只允许 1 条初始首帧提示词：当前 {} 条",
            first_frame_prompts.len()
        )));
    }
    let first_segment = segment_plan.first().ok_or_else(|| {
        AppError::InvalidArgument("首帧串联片段规划至少需要 1 个片段".to_string())
    })?;
    let frame = &first_frame_prompts[0];
    if frame.frame_number != 1 {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词 frame_number 必须为 1，实际为 {}",
            frame.frame_number
        )));
    }
    if frame.segment_numbers != vec![first_segment.segment_number] {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词 segment_numbers 必须且只能包含第 1 个片段编号 {}",
            first_segment.segment_number
        )));
    }
    if frame.duration_secs < 1 {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词时长必须大于 0，实际为 {}s",
            frame.duration_secs
        )));
    }
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        "初始首帧",
    )?;
    validate_reference_label_text(
        &frame.frame_prompt,
        "初始首帧 frame_prompt",
        frame.reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &frame.negative_prompt,
        "初始首帧 negative_prompt",
        frame.reference_image_ids.len(),
    )?;
    ensure_no_storyboard_panels(frame, "初始首帧")?;
    Ok(())
}

pub(in crate::video) fn validate_storyboard_prompts(
    storyboard_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    let expected_count = expected_video_unit_count(segment_plan);
    if storyboard_prompts.len() != expected_count {
        return Err(AppError::InvalidArgument(format!(
            "分镜提示词数量必须等于视频单元数 2n-1：当前 {} 条，片段数 {}，应为 {} 条",
            storyboard_prompts.len(),
            segment_plan.len(),
            expected_count
        )));
    }
    for (idx, frame) in storyboard_prompts.iter().enumerate() {
        let expected_number = (idx + 1) as i32;
        if frame.frame_number != expected_number {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 frame_number 必须从 1 连续编号，位置 {} 应为 {}，实际为 {}",
                idx + 1,
                expected_number,
                frame.frame_number
            )));
        }
        let expected_segment_numbers =
            storyboard_expected_segment_numbers(segment_plan, frame.frame_number)?;
        let representative_segment_number =
            expected_segment_numbers.first().copied().ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "分镜提示词 #{} 缺少对应片段规划",
                    frame.frame_number
                ))
            })?;
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == representative_segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "分镜提示词 #{} 缺少对应片段规划",
                    frame.frame_number
                ))
            })?;
        if frame.segment_numbers != expected_segment_numbers {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
                frame.frame_number, expected_segment_numbers, frame.segment_numbers
            )));
        }
        if frame.duration_secs < 1 {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} 时长必须大于 0，实际为 {}s",
                frame.frame_number, frame.duration_secs
            )));
        }
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} aspect_ratio 必须为 {}，实际为 {}",
                frame.frame_number, segment.aspect_ratio, frame.aspect_ratio
            )));
        }
        ensure_video_reference_ids_known(
            &frame.reference_image_ids,
            allowed_reference_ids,
            &format!("视频分镜 #{}", frame.frame_number),
        )?;
        validate_reference_label_text(
            &frame.frame_prompt,
            &format!("视频分镜 #{} frame_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_reference_label_text(
            &frame.negative_prompt,
            &format!("视频分镜 #{} negative_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_storyboard_panels(frame)?;
    }
    Ok(())
}

fn ensure_no_storyboard_panels(frame: &VideoFramePrompt, label: &str) -> Result<(), AppError> {
    if !frame.storyboard_panels.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{label} 不能输出 storyboard_panels；只有分镜模式的分镜板提示词可以输出多格分镜结构"
        )));
    }
    Ok(())
}

fn validate_storyboard_panels(frame: &VideoFramePrompt) -> Result<(), AppError> {
    let panel_count = frame.storyboard_panels.len();
    if !(2..=6).contains(&panel_count) {
        return Err(AppError::InvalidArgument(format!(
            "视频分镜 #{} storyboard_panels 数量必须在 2 到 6 格之间，实际为 {}",
            frame.frame_number, panel_count
        )));
    }
    for (index, panel) in frame.storyboard_panels.iter().enumerate() {
        validate_storyboard_panel(frame.frame_number, index + 1, panel)?;
        if !frame_prompt_has_panel_marker(&frame.frame_prompt, index + 1) {
            return Err(AppError::InvalidArgument(format!(
                "视频分镜 #{} frame_prompt 必须逐格描述分镜板内容，缺少第 {} 格标记",
                frame.frame_number,
                index + 1
            )));
        }
    }
    Ok(())
}

fn validate_storyboard_panel(
    frame_number: i32,
    expected_index: usize,
    panel: &StoryboardPanel,
) -> Result<(), AppError> {
    if panel.panel_index != expected_index as i32 {
        return Err(AppError::InvalidArgument(format!(
            "视频分镜 #{} storyboard_panels[{}].panel_index 必须为 {}，实际为 {}",
            frame_number,
            expected_index - 1,
            expected_index,
            panel.panel_index
        )));
    }
    for (field, value) in [
        ("moment", panel.moment.as_str()),
        ("visual_goal", panel.visual_goal.as_str()),
        ("composition", panel.composition.as_str()),
        ("continuity_note", panel.continuity_note.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频分镜 #{} storyboard_panels[{}].{} 不能为空",
                frame_number,
                expected_index - 1,
                field
            )));
        }
    }
    Ok(())
}

fn frame_prompt_has_panel_marker(prompt: &str, panel_index: usize) -> bool {
    let compact = prompt.replace(' ', "");
    let lower = prompt.to_ascii_lowercase();
    compact.contains(&format!("第{panel_index}格"))
        || compact.contains(&format!("第{panel_index}个画格"))
        || lower.contains(&format!("panel {panel_index}"))
        || lower.contains(&format!("panel{panel_index}"))
}

pub(in crate::video) fn validate_video_prompts(
    video_prompts: &[VideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if video_prompts.len() != segment_by_number.len() {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词数量必须等于片段数：当前 {} 条，片段数 {}",
            video_prompts.len(),
            segment_by_number.len()
        )));
    }
    validate_partial_video_prompts(video_prompts, segment_by_number, prompt_output_language)
}

pub(in crate::video) fn validate_partial_video_prompts(
    video_prompts: &[VideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt in video_prompts {
        let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 必须对应已有片段编号",
                prompt.shot_number
            ))
        })?;
        validate_video_prompt_shot_against_segment(prompt.clone(), segment, prompt_output_language)
            .map_err(|err| match err {
                AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                    "视频提示词 shot_number {} 无效：{}",
                    prompt.shot_number, message
                )),
                other => other,
            })?;
        if !seen.insert(prompt.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 重复",
                prompt.shot_number
            )));
        }
    }
    Ok(())
}

pub(in crate::video) fn validate_chat_video_prompts(
    video_prompts: &[ChatVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if video_prompts.len() != segment_by_number.len() {
        return Err(AppError::InvalidArgument(format!(
            "Chat 视频提示词数量必须等于片段数：当前 {} 条，片段数 {}",
            video_prompts.len(),
            segment_by_number.len()
        )));
    }
    validate_partial_chat_video_prompts(video_prompts, segment_by_number, prompt_output_language)
}

pub(in crate::video) fn validate_partial_chat_video_prompts(
    video_prompts: &[ChatVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt in video_prompts {
        let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 必须对应已有片段编号",
                prompt.shot_number
            ))
        })?;
        validate_chat_video_prompt_shot_against_segment(
            prompt.clone(),
            segment,
            prompt_output_language,
        )
        .map_err(|err| match err {
            AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 无效：{}",
                prompt.shot_number, message
            )),
            other => other,
        })?;
        if !seen.insert(prompt.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 重复",
                prompt.shot_number
            )));
        }
    }
    Ok(())
}

pub(in crate::video) fn reusable_boundary_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    reusable_video_prompts_from_rows(rows, "边界帧视频提示词")
}

pub(in crate::video) fn reusable_first_frame_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    reusable_video_prompts_from_rows(rows, "首帧串联视频提示词")
}

pub(in crate::video) fn reusable_chat_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, ChatVideoPromptShot>, AppError> {
    let mut by_shot = BTreeMap::<i32, ChatVideoPromptShot>::new();
    for row in rows {
        let prompt = reusable_chat_video_prompt_from_row(row)?;
        by_shot.entry(row.shot_number).or_insert(prompt);
    }
    Ok(by_shot)
}

fn reusable_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
    label: &str,
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    let mut by_shot = BTreeMap::<i32, VideoPromptShot>::new();
    for row in rows {
        let prompt = reusable_video_prompt_from_row(row, label)?;
        by_shot.entry(row.shot_number).or_insert(prompt);
    }
    Ok(by_shot)
}

fn reusable_video_prompt_from_row(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<VideoPromptShot, AppError> {
    let metadata = row_metadata::<SavedVideoPromptMetadata>(row, label)?;
    let motion_prompt = row.motion_prompt.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 motion_prompt",
            row.shot_number
        ))
    })?;
    let aspect_ratio = required_row_aspect_ratio(row, label)?;
    let reference_image_ids = row_reference_image_ids(row, label)?;
    Ok(VideoPromptShot {
        shot_number: row.shot_number,
        title: required_row_text(&row.title, row, label, "title")?,
        scene_summary: required_row_text(&row.scene_summary, row, label, "scene_summary")?,
        self_contained_context: metadata.self_contained_context,
        start_state: metadata.start_state,
        end_state: metadata.end_state,
        transition_in: metadata.transition_in,
        visual_prompt: row.visual_prompt.clone(),
        motion_prompt,
        negative_prompt: row.negative_prompt.clone().unwrap_or_default(),
        blocking_plan: metadata.blocking_plan,
        lighting_plan: metadata.lighting_plan,
        emotion_action_beats: metadata.emotion_action_beats,
        dialogue_beats: metadata.dialogue_beats,
        cinematography_plan: metadata.cinematography_plan,
        temporal_beats: metadata.temporal_beats,
        reference_binding: metadata.reference_binding,
        continuity_check: metadata.continuity_check,
        duration_secs: row.duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
}

fn required_row_aspect_ratio(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<String, AppError> {
    row.aspect_ratio.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 aspect_ratio",
            row.shot_number
        ))
    })
}

fn row_reference_image_ids(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<Vec<i64>, AppError> {
    match row.reference_image_ids_json.as_ref() {
        Some(value) => serde_json::from_value::<Vec<i64>>(value.clone()).map_err(|err| {
            AppError::InvalidArgument(format!(
                "已保存{label} #{} 的 reference_image_ids_json 无效：{}",
                row.shot_number, err
            ))
        }),
        None => Ok(Vec::new()),
    }
}

fn required_row_text(
    value: &Option<String>,
    row: &chapter_video_prompt::Model,
    label: &str,
    field: &str,
) -> Result<String, AppError> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("已保存{label} #{} 缺少 {field}", row.shot_number))
        })
}

fn reusable_chat_video_prompt_from_row(
    row: &chapter_video_prompt::Model,
) -> Result<ChatVideoPromptShot, AppError> {
    let metadata = row_metadata::<SavedChatVideoPromptMetadata>(row, "Chat 视频提示词")?;
    let aspect_ratio = required_row_aspect_ratio(row, "Chat 视频提示词")?;
    let reference_image_ids = row_reference_image_ids(row, "Chat 视频提示词")?;
    Ok(ChatVideoPromptShot {
        shot_number: row.shot_number,
        title: required_row_text(&row.title, row, "Chat 视频提示词", "title")?,
        scene_summary: required_row_text(
            &row.scene_summary,
            row,
            "Chat 视频提示词",
            "scene_summary",
        )?,
        chat_prompt: row.visual_prompt.clone(),
        negative_prompt: row.negative_prompt.clone().unwrap_or_default(),
        chat_video: metadata.chat_video,
        duration_secs: row.duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
}

#[derive(Deserialize)]
struct SavedVideoPromptMetadata {
    #[serde(rename = "video_unit")]
    _video_unit: Value,
    self_contained_context: Value,
    start_state: Value,
    end_state: Value,
    transition_in: Value,
    blocking_plan: Value,
    lighting_plan: Value,
    emotion_action_beats: Value,
    dialogue_beats: Value,
    cinematography_plan: Value,
    temporal_beats: Value,
    reference_binding: Value,
    continuity_check: Value,
}

#[derive(Deserialize)]
struct SavedChatVideoPromptMetadata {
    #[serde(rename = "video_unit")]
    _video_unit: Value,
    chat_video: Value,
}

fn row_metadata<T>(row: &chapter_video_prompt::Model, label: &str) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let value = row.metadata_json.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 metadata_json",
            row.shot_number
        ))
    })?;
    serde_json::from_value(value).map_err(|err| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 的 metadata_json 无效：{}",
            row.shot_number, err
        ))
    })
}

pub(super) fn prompt_row_draft(
    row: &chapter_video_prompt::Model,
) -> Result<NewChapterVideoPrompt, AppError> {
    Ok(new_prompt_draft(NewChapterVideoPromptInput {
        project_id: row.project_id,
        chapter_id: row.chapter_id,
        workflow_mode: PromptWorkflowMode::from_row(row)?,
        style: row.style.as_deref(),
        prompt_type: &row.prompt_type,
        shot_number: row.shot_number,
        title: row.title.as_deref(),
        scene_summary: row.scene_summary.as_deref(),
        visual_prompt: &row.visual_prompt,
        motion_prompt: row.motion_prompt.as_deref(),
        negative_prompt: row.negative_prompt.as_deref(),
        duration_secs: row.duration_secs,
        aspect_ratio: row.aspect_ratio.as_deref(),
        reference_image_ids_json: row.reference_image_ids_json.clone(),
        metadata_json: row.metadata_json.clone(),
    }))
}

#[derive(Clone, Copy)]
pub(super) struct VideoPromptDraftCommonInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub source_metadata: &'a Value,
    pub available_assets: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
}

pub(super) struct FramePromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_prompt_type: &'a str,
    pub frame_prompts: &'a [VideoFramePrompt],
}

pub(super) fn frame_prompt_drafts(input: FramePromptDraftsInput<'_>) -> Vec<NewChapterVideoPrompt> {
    let FramePromptDraftsInput {
        common,
        workflow_mode,
        prompt_type,
        source_prompt_type,
        frame_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    frame_prompts
        .iter()
        .map(|frame| {
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                    "boundary_frame": frame.clone(),
                    "video_frame_unit": {
                        "frame_role": if frame.frame_number % 2 == 1 { "segment_start" } else { "segment_end" },
                        "source_segment_number": frame.segment_numbers.first().copied()
                    },
                    "reference_slots": super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode,
                style: Some(style),
                prompt_type,
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}

pub(super) struct FirstFramePromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub first_frame_prompts: &'a [VideoFramePrompt],
}

pub(super) fn first_frame_prompt_drafts(
    input: FirstFramePromptDraftsInput<'_>,
) -> Vec<NewChapterVideoPrompt> {
    let FirstFramePromptDraftsInput {
        common,
        source_prompt_type,
        first_frame_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    first_frame_prompts
        .iter()
        .map(|frame| {
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                    "first_frame": frame.clone(),
                    "reference_slots": super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some(style),
                prompt_type: "first_frame_prompt",
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}

pub(super) struct StoryboardPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub video_units: &'a [VideoUnitPlan],
    pub storyboard_prompts: &'a [VideoFramePrompt],
}

pub(super) fn storyboard_prompt_drafts(
    input: StoryboardPromptDraftsInput<'_>,
) -> Vec<NewChapterVideoPrompt> {
    let StoryboardPromptDraftsInput {
        common,
        source_prompt_type,
        video_units,
        storyboard_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    storyboard_prompts
        .iter()
        .map(|frame| {
            let video_unit = video_units
                .iter()
                .find(|unit| unit.unit_number == frame.frame_number);
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": STORYBOARD_PROMPT_CONTRACT_VERSION,
                    "storyboard_frame": frame.clone(),
                    "video_unit": video_unit,
                    "reference_slots": super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::Storyboard,
                style: Some(style),
                prompt_type: "storyboard_prompt",
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}

pub(super) struct VideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(super) fn video_prompt_drafts(
    input: VideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let VideoPromptDraftsInput {
        common,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        video_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let start_frame_number = video_unit.start_frame_number.ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 对应视频单元缺少起始边界帧编号",
                    prompt.shot_number
                ))
            })?;
            let end_frame_number = video_unit.end_frame_number.ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 对应视频单元缺少结束边界帧编号",
                    prompt.shot_number
                ))
            })?;
            let start_boundary_frame =
                frame_by_number.get(&start_frame_number).ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "视频提示词 #{} 缺少起始边界帧 #{}",
                        prompt.shot_number, start_frame_number
                    ))
                })?;
            let end_boundary_frame = frame_by_number.get(&end_frame_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 缺少结束边界帧 #{}",
                    prompt.shot_number, end_frame_number
                ))
            })?;
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some(style),
                prompt_type: "video_prompt",
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.visual_prompt,
                motion_prompt: Some(&prompt.motion_prompt),
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_boundary_frame": start_boundary_frame,
                        "end_boundary_frame": end_boundary_frame,
                        "available_assets": available_assets.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "self_contained_context": prompt.self_contained_context.clone(),
                        "start_state": prompt.start_state.clone(),
                        "end_state": prompt.end_state.clone(),
                        "transition_in": prompt.transition_in.clone(),
                        "blocking_plan": prompt.blocking_plan.clone(),
                        "lighting_plan": prompt.lighting_plan.clone(),
                        "emotion_action_beats": prompt.emotion_action_beats.clone(),
                        "dialogue_beats": prompt.dialogue_beats.clone(),
                        "cinematography_plan": prompt.cinematography_plan.clone(),
                        "temporal_beats": prompt.temporal_beats.clone(),
                        "reference_binding": prompt.reference_binding.clone(),
                        "continuity_check": prompt.continuity_check.clone(),
                        "source_prompt_type": "frame_segment_plan",
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}

pub(super) struct FirstFrameVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub first_frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(super) fn first_frame_video_prompt_drafts(
    input: FirstFrameVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let FirstFrameVideoPromptDraftsInput {
        common,
        source_prompt_type,
        segment_by_number,
        video_unit_by_number,
        first_frame_by_number,
        video_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    let generated_first_frame = first_frame_by_number.get(&1).ok_or_else(|| {
        AppError::InvalidArgument("首帧串联视频提示词缺少初始首帧 #1".to_string())
    })?;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "首帧串联视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "首帧串联视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let start_first_frame = if prompt.shot_number == 1 {
                json!(generated_first_frame)
            } else {
                Value::Null
            };
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some(style),
                prompt_type: "video_prompt",
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.visual_prompt,
                motion_prompt: Some(&prompt.motion_prompt),
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_first_frame": start_first_frame,
                        "available_assets": available_assets.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "self_contained_context": prompt.self_contained_context.clone(),
                        "start_state": prompt.start_state.clone(),
                        "end_state": prompt.end_state.clone(),
                        "transition_in": prompt.transition_in.clone(),
                        "blocking_plan": prompt.blocking_plan.clone(),
                        "lighting_plan": prompt.lighting_plan.clone(),
                        "emotion_action_beats": prompt.emotion_action_beats.clone(),
                        "dialogue_beats": prompt.dialogue_beats.clone(),
                        "cinematography_plan": prompt.cinematography_plan.clone(),
                        "temporal_beats": prompt.temporal_beats.clone(),
                        "reference_binding": prompt.reference_binding.clone(),
                        "continuity_check": prompt.continuity_check.clone(),
                        "source_prompt_type": source_prompt_type,
                        "first_frame_chain": {
                            "start_source": if prompt.shot_number == 1 {
                                "generated_first_frame"
                            } else {
                                "previous_video_tail_frame"
                            },
                            "first_frame_prompt_number": if prompt.shot_number == 1 {
                                Value::from(1)
                            } else {
                                Value::Null
                            },
                            "previous_video_shot_number": if prompt.shot_number > 1 {
                                Value::from(prompt.shot_number - 1)
                            } else {
                                Value::Null
                            }
                        },
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}

pub(super) struct ChatVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub generation_reference_ids: &'a Value,
    pub video_prompts: &'a [ChatVideoPromptShot],
}

pub(super) fn chat_video_prompt_drafts(
    input: ChatVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let ChatVideoPromptDraftsInput {
        common,
        source_prompt_type,
        segment_by_number,
        video_unit_by_number,
        generation_reference_ids,
        video_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "Chat 视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Chat 视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::ChatVideo,
                style: Some(style),
                prompt_type: "video_prompt",
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.chat_prompt,
                motion_prompt: None,
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": CHAT_VIDEO_CONTRACT_VERSION,
                        "chat_video": prompt.chat_video.clone(),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "available_assets": available_assets.clone(),
                        "generation_reference_image_ids": generation_reference_ids.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "source_prompt_type": source_prompt_type,
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}

pub(super) struct StoryboardVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub storyboard_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(super) fn storyboard_video_prompt_drafts(
    input: StoryboardVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let StoryboardVideoPromptDraftsInput {
        common,
        segment_by_number,
        video_unit_by_number,
        storyboard_by_number,
        video_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    } = common;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "分镜视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let storyboard = storyboard_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜视频提示词 #{} 缺少对应分镜提示词",
                        prompt.shot_number
                    ))
                })?;
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::Storyboard,
                style: Some(style),
                prompt_type: "video_prompt",
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.visual_prompt,
                motion_prompt: Some(&prompt.motion_prompt),
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "storyboard_frame": storyboard,
                        "storyboard": {
                            "prompt_number": prompt.shot_number,
                            "image_id": prompt.reference_image_ids.first().copied()
                        },
                        "available_assets": available_assets.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "self_contained_context": prompt.self_contained_context.clone(),
                        "start_state": prompt.start_state.clone(),
                        "end_state": prompt.end_state.clone(),
                        "transition_in": prompt.transition_in.clone(),
                        "blocking_plan": prompt.blocking_plan.clone(),
                        "lighting_plan": prompt.lighting_plan.clone(),
                        "emotion_action_beats": prompt.emotion_action_beats.clone(),
                        "dialogue_beats": prompt.dialogue_beats.clone(),
                        "cinematography_plan": prompt.cinematography_plan.clone(),
                        "temporal_beats": prompt.temporal_beats.clone(),
                        "reference_binding": prompt.reference_binding.clone(),
                        "continuity_check": prompt.continuity_check.clone(),
                        "source_prompt_type": "frame_segment_plan",
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}

fn optional_prompt_text(value: &str) -> Option<&str> {
    let text = value.trim();
    if text.is_empty() || matches!(text, "无" | "无特殊负面词" | "无特殊要求") {
        None
    } else {
        Some(text)
    }
}

fn compact_shot_metadata(prompt: &VideoPromptShot) -> Value {
    json!({
        "shot_number": prompt.shot_number,
        "title": prompt.title.as_str(),
        "scene_summary": prompt.scene_summary.as_str(),
        "self_contained_context": prompt.self_contained_context.clone(),
        "start_state": prompt.start_state.clone(),
        "end_state": prompt.end_state.clone(),
        "transition_in": prompt.transition_in.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn segment(segment_number: i32) -> VideoSegmentPlan {
        VideoSegmentPlan {
            segment_number,
            analysis_shot_id: segment_number,
            segment_index_in_shot: 1,
            shot_segment_count: 1,
            covered_beat_ids: vec![1],
            title: format!("片段 {segment_number}"),
            scene_summary: "片段摘要".to_string(),
            blocking_summary: "站位摘要".to_string(),
            spatial_anchors: json!({"anchor": "door"}),
            emotion_action_goals: json!(["目标"]),
            video_intent: json!({
                "hook": "片段钩子",
                "audience_effect": "观众感受",
                "rhythm": "稳定",
                "visual_priority": "主体动作"
            }),
            emotional_arc: json!({
                "start": "开始",
                "turn": "转折",
                "end": "结束",
                "function": "测试"
            }),
            performance_plan: json!({
                "movement_path": "从起点移动到终点",
                "body_performance": "身体姿态随动作变化",
                "gaze_and_hands": "视线和手部跟随目标变化",
                "pause_or_reaction": "结尾停顿",
                "camera_relationship": "镜头保持主体动作清晰",
                "time_blocks": [],
                "action_script": []
            }),
            material_coverage: json!({
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "无"
            }),
            source_span_ids: vec!["ch1:s1:f1".to_string()],
            duration_secs: 6,
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
        }
    }

    fn frame(frame_number: i32) -> VideoFramePrompt {
        VideoFramePrompt {
            frame_number,
            title: format!("帧 {frame_number}"),
            scene_summary: "帧摘要".to_string(),
            storyboard_panels: Vec::new(),
            frame_prompt: "完整帧提示词".to_string(),
            blocking_state: json!({"blocking": "stable"}),
            lighting_state: json!({"lighting": "warm"}),
            negative_prompt: "不要变形".to_string(),
            duration_secs: 6,
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
            segment_numbers: vec![frame_number.max(1)],
        }
    }

    fn storyboard_panel(panel_index: i32) -> StoryboardPanel {
        StoryboardPanel {
            panel_index,
            moment: format!("第 {panel_index} 个状态"),
            visual_goal: "表达可见动作变化".to_string(),
            composition: "保持主体和空间关系清晰".to_string(),
            continuity_note: "延续人物、场景和光源方向".to_string(),
        }
    }

    fn video_unit_for_segment(segment: &VideoSegmentPlan) -> VideoUnitPlan {
        super::super::video_units_for_segment_plan(
            std::slice::from_ref(segment),
            segment.duration_secs,
        )
        .remove(0)
    }

    fn shot(shot_number: i32) -> VideoPromptShot {
        VideoPromptShot {
            shot_number,
            title: format!("镜头 {shot_number}"),
            scene_summary: "镜头摘要".to_string(),
            self_contained_context: json!({
                "must_repeat_from_previous": ["人物身份", "空间锚点"],
                "must_establish_in_segment": ["本段动作目标"],
                "must_preserve_throughout": ["光影方向"],
                "handoff_to_next": "结尾状态",
                "omission_risks": ["上下段断裂"]
            }),
            start_state: json!({"state": "start"}),
            end_state: json!({"state": "end"}),
            transition_in: json!({
                "from_previous_end": "承接上一状态",
                "to_current_start": "进入本段起点",
                "cut_type": "连续承接",
                "camera_transition": "稳定衔接",
                "first_beat_action": "人物开始动作"
            }),
            visual_prompt: "完整 visual prompt".to_string(),
            motion_prompt: "完整 motion prompt".to_string(),
            negative_prompt: "不要变形".to_string(),
            blocking_plan: json!({"blocking": "plan"}),
            lighting_plan: json!({"lighting": "plan"}),
            emotion_action_beats: json!(["beat"]),
            dialogue_beats: json!([]),
            cinematography_plan: json!({"camera": "push"}),
            temporal_beats: json!(["start", "end"]),
            reference_binding: json!({"bindings": [], "coverage_check": "参考一致性由文字约束承担"}),
            continuity_check: json!({"revision": "无需修订"}),
            duration_secs: 6,
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
        }
    }

    fn test_metadata() -> Value {
        json!({"source": {"content_sha256": "current"}})
    }

    fn test_common_input<'a>(
        source_metadata: &'a Value,
        available_assets: &'a Value,
        generation_asset_fingerprint: &'a Value,
    ) -> VideoPromptDraftCommonInput<'a> {
        VideoPromptDraftCommonInput {
            project_id: 1,
            chapter_id: 2,
            style: "电影感写实",
            prompt_output_language: PromptOutputLanguage::ZhCn,
            source_metadata,
            available_assets,
            generation_asset_fingerprint,
        }
    }

    fn expect_invalid_argument(result: Result<Vec<NewChapterVideoPrompt>, AppError>) -> String {
        match result {
            Ok(_) => panic!("draft construction should fail"),
            Err(AppError::InvalidArgument(message)) => message,
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn boundary_frame_prompts_reject_storyboard_panels() {
        let segment = segment(1);
        let mut start_frame = frame(1);
        start_frame.segment_numbers = vec![1];
        start_frame.storyboard_panels = vec![storyboard_panel(1), storyboard_panel(2)];
        let mut end_frame = frame(2);
        end_frame.segment_numbers = vec![1];

        let err = validate_frame_prompts(&[start_frame, end_frame], &[segment], &HashSet::new())
            .expect_err("boundary frames must reject storyboard panels");

        assert!(
            matches!(&err, AppError::InvalidArgument(message) if message.contains("不能输出 storyboard_panels")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn storyboard_prompts_accept_dynamic_panel_count() {
        let segment = segment(1);
        let mut frame = frame(1);
        frame.segment_numbers = vec![1];
        frame.storyboard_panels = vec![
            storyboard_panel(1),
            storyboard_panel(2),
            storyboard_panel(3),
        ];
        frame.frame_prompt =
            "第 1 格：建立起始状态。\n第 2 格：表现核心动作。\n第 3 格：稳定到结尾状态。"
                .to_string();

        validate_storyboard_prompts(&[frame], &[segment], &HashSet::new())
            .expect("valid storyboard panels should pass");
    }

    #[test]
    fn storyboard_prompts_require_frame_prompt_panel_markers() {
        let segment = segment(1);
        let mut frame = frame(1);
        frame.segment_numbers = vec![1];
        frame.storyboard_panels = vec![storyboard_panel(1), storyboard_panel(2)];
        frame.frame_prompt = "第 1 格：建立起始状态。第二格表现核心动作。".to_string();

        let err = validate_storyboard_prompts(&[frame], &[segment], &HashSet::new())
            .expect_err("missing panel marker should fail");

        assert!(
            matches!(&err, AppError::InvalidArgument(message) if message.contains("缺少第 2 格标记")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn video_prompt_drafts_reject_missing_boundary_frame_metadata() {
        let segment = segment(1);
        let mut segments = HashMap::new();
        segments.insert(1, &segment);
        let mut video_units = HashMap::new();
        video_units.insert(1, video_unit_for_segment(&segment));
        let mut frames = HashMap::new();
        frames.insert(1, frame(1));

        let source_metadata = test_metadata();
        let available_assets = json!({});
        let generation_asset_fingerprint = json!({"schema_version": 1});
        let message = expect_invalid_argument(video_prompt_drafts(VideoPromptDraftsInput {
            common: test_common_input(
                &source_metadata,
                &available_assets,
                &generation_asset_fingerprint,
            ),
            segment_by_number: &segments,
            video_unit_by_number: &video_units,
            frame_by_number: &frames,
            video_prompts: &[shot(1)],
        }));

        assert!(message.contains("缺少结束边界帧 #2"));
    }

    #[test]
    fn first_frame_video_prompt_drafts_reject_missing_initial_first_frame() {
        let segment = segment(1);
        let mut segments = HashMap::new();
        segments.insert(1, &segment);
        let mut video_units = HashMap::new();
        video_units.insert(1, video_unit_for_segment(&segment));
        let frames = HashMap::new();

        let source_metadata = test_metadata();
        let available_assets = json!({});
        let generation_asset_fingerprint = json!({"schema_version": 1});
        let message = expect_invalid_argument(first_frame_video_prompt_drafts(
            FirstFrameVideoPromptDraftsInput {
                common: test_common_input(
                    &source_metadata,
                    &available_assets,
                    &generation_asset_fingerprint,
                ),
                source_prompt_type: "frame_segment_plan",
                segment_by_number: &segments,
                video_unit_by_number: &video_units,
                first_frame_by_number: &frames,
                video_prompts: &[shot(1)],
            },
        ));

        assert!(message.contains("缺少初始首帧 #1"));
    }
}
