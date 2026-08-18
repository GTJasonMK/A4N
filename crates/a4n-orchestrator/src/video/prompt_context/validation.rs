use std::collections::HashSet;

use a4n_core::error::AppError;
use a4n_domain::VideoAspectRatio;

use super::super::ChapterVideoPromptOptions;
use super::super::library::{MAX_REFERENCE_IMAGE_IDS, MAX_VIDEO_LIBRARY_IMAGE_IDS};

const MAX_DURATION_OPTION_COUNT: usize = 12;

pub(in crate::video) fn validate_prompt_options(
    options: ChapterVideoPromptOptions,
) -> Result<ChapterVideoPromptOptions, AppError> {
    validate_prompt_options_with_reference_limit(options, MAX_REFERENCE_IMAGE_IDS)
}

pub fn validate_prompt_options_with_reference_limit(
    options: ChapterVideoPromptOptions,
    max_reference_image_ids: usize,
) -> Result<ChapterVideoPromptOptions, AppError> {
    let style = options.style.trim();
    if style.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 style 不能为空".to_string(),
        ));
    }
    if options.prompt_batch_size < 1 {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 prompt_batch_size 必须大于等于 1".to_string(),
        ));
    }
    let prompt_output_language = options.prompt_output_language;
    if !(1..=60).contains(&options.duration_secs) {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    let duration_secs_options =
        normalize_duration_secs_options(options.duration_secs, &options.duration_secs_options)?;
    if let Some(capability) = options.duration_capability.as_ref() {
        capability.validate_duration(options.duration_secs)?;
        capability.validate_duration_options(&duration_secs_options)?;
    }
    options
        .aspect_ratio
        .parse::<VideoAspectRatio>()
        .map_err(|_| {
            AppError::InvalidArgument(format!(
                "视频提示词生成 aspect_ratio 无效: {}，必须是 {}",
                options.aspect_ratio,
                VideoAspectRatio::supported_values()
            ))
        })?;
    validate_positive_unique_ids("reference_image_ids", &options.reference_image_ids)?;
    validate_positive_unique_ids("video_library_image_ids", &options.video_library_image_ids)?;
    validate_id_count(
        "reference_image_ids",
        options.reference_image_ids.len(),
        max_reference_image_ids,
    )?;
    validate_id_count(
        "video_library_image_ids",
        options.video_library_image_ids.len(),
        MAX_VIDEO_LIBRARY_IMAGE_IDS,
    )?;
    if options.target_shot_number.is_some_and(|value| value <= 0) {
        return Err(AppError::InvalidArgument(
            "target_shot_number 必须为正整数".to_string(),
        ));
    }
    if options
        .segment_plan_prompt_id
        .is_some_and(|value| value <= 0)
    {
        return Err(AppError::InvalidArgument(
            "segment_plan_prompt_id 必须为正整数".to_string(),
        ));
    }
    Ok(ChapterVideoPromptOptions {
        style: style.to_string(),
        prompt_output_language,
        prompt_batch_size: options.prompt_batch_size,
        segment_plan_prompt_id: options.segment_plan_prompt_id,
        target_shot_number: options.target_shot_number,
        target_narrative_unit_index: options.target_narrative_unit_index,
        segment_recipe: options.segment_recipe,
        duration_secs: options.duration_secs,
        duration_secs_options,
        aspect_ratio: options.aspect_ratio,
        reference_image_ids: options.reference_image_ids,
        video_library_image_ids: options.video_library_image_ids,
        duration_capability: options.duration_capability,
        regenerate_existing: options.regenerate_existing,
        workflow_run_id: options.workflow_run_id,
    })
}

fn normalize_duration_secs_options(
    default_duration_secs: i32,
    duration_secs_options: &[i32],
) -> Result<Vec<i32>, AppError> {
    let mut options = if duration_secs_options.is_empty() {
        vec![default_duration_secs]
    } else {
        duration_secs_options.to_vec()
    };
    if !options.contains(&default_duration_secs) {
        options.push(default_duration_secs);
    }
    options.sort_unstable();
    options.dedup();
    if options.len() > MAX_DURATION_OPTION_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词生成 duration_secs_options 最多选择 {MAX_DURATION_OPTION_COUNT} 个候选时长"
        )));
    }
    for duration_secs in &options {
        if !(1..=60).contains(duration_secs) {
            return Err(AppError::InvalidArgument(
                "视频提示词生成 duration_secs_options 必须全部在 1 到 60 秒之间".to_string(),
            ));
        }
    }
    Ok(options)
}

fn validate_id_count(field: &str, count: usize, max: usize) -> Result<(), AppError> {
    if count > max {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词生成 {field} 最多选择 {max} 张"
        )));
    }
    Ok(())
}

fn validate_positive_unique_ids(field: &str, ids: &[i64]) -> Result<(), AppError> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词生成 {field}[{index}] 必须为正整数"
            )));
        }
        if !seen.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词生成 {field} 包含重复 ID：{id}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
