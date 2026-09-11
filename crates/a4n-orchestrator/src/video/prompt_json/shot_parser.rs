use std::collections::{HashMap, HashSet};

use a4n_error::AppError;

use a4n_domain::prompt_language::PromptOutputLanguage;

use super::contracts::{VideoPromptResponse, VideoPromptShot, VideoSegmentPlan};
use super::derived_fields::video_prompt_shot_from_input;
use super::{parse_strict_json_response, validate_model_video_prompt_shot_against_segment};

pub(in crate::video) fn parse_video_prompt_shots(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoPromptShot>, AppError> {
    let response: VideoPromptResponse = parse_strict_json_response(raw, "视频提示词")?;
    if response.shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词响应必须包含至少一个 shots 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let ordered_segment_numbers = ordered_segment_numbers(segment_by_number);
    let shot_count = response.shots.len();
    let mut shots = Vec::with_capacity(response.shots.len());
    for (index, input) in response.shots.into_iter().enumerate() {
        let shot_number = derived_prompt_shot_number(
            index,
            shot_count,
            &ordered_segment_numbers,
            "视频提示词 shot_number",
        )?;
        let segment = segment_by_number.get(&shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 必须对应已有片段编号",
                shot_number
            ))
        })?;
        let shot = video_prompt_shot_from_input(input, shot_number, segment);
        let shot =
            validate_model_video_prompt_shot_against_segment(shot, segment, prompt_output_language)
                .map_err(|err| match err {
                    AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                        "视频提示词 shot_number {} 无效：{}",
                        shot_number, message
                    )),
                    other => other,
                })?;
        if !seen_numbers.insert(shot.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 重复",
                shot.shot_number
            )));
        }
        shots.push(shot);
    }
    Ok(shots)
}

pub(in crate::video) fn ordered_segment_numbers(
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
) -> Vec<i32> {
    let mut numbers = segment_by_number.keys().copied().collect::<Vec<_>>();
    numbers.sort_unstable();
    numbers
}

pub(in crate::video) fn derived_prompt_shot_number(
    index: usize,
    item_count: usize,
    ordered_segment_numbers: &[i32],
    field: &str,
) -> Result<i32, AppError> {
    if item_count == 1 && ordered_segment_numbers.len() == 1 {
        return Ok(ordered_segment_numbers[0]);
    }
    if item_count == ordered_segment_numbers.len() {
        return ordered_segment_numbers
            .get(index)
            .copied()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 无法按片段顺序推导")));
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 缺失，且无法从当前批次和片段数量唯一推导"
    )))
}
