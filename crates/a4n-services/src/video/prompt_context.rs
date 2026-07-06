use std::collections::HashSet;

use serde_json::Value;

use a4n_core::error::AppError;

use super::library::{MAX_REFERENCE_IMAGE_IDS, MAX_VIDEO_LIBRARY_IMAGE_IDS};
use super::{ChapterVideoPromptOptions, ChapterVideoSourceContext};
use crate::prompt_language::PromptOutputLanguage;
use crate::source::{ChapterSourceContext, source_metadata_with_flags};

mod builders;
mod ledger;
mod project_context;
mod public_context;

const SUPPORTED_VIDEO_ASPECT_RATIOS: &[&str] = &["21:9", "16:9", "4:3", "1:1", "3:4", "9:16"];
const MAX_DURATION_OPTION_COUNT: usize = 12;

pub(super) use self::builders::{
    FinalVideoPromptContext, FramePromptContext, VideoPlanningPromptContext,
    build_beat_extraction_user, build_boundary_video_prompt_review_user,
    build_boundary_video_prompt_user, build_chat_video_prompt_review_user,
    build_chat_video_prompt_user, build_first_frame_prompt_user,
    build_first_frame_video_prompt_review_user, build_first_frame_video_prompt_user,
    build_segment_plan_for_shot_user, build_shot_grouping_user, build_single_frame_prompt_user,
    build_storyboard_prompt_user, build_storyboard_video_prompt_review_user,
    build_storyboard_video_prompt_user,
};
pub(super) use self::ledger::{
    VIDEO_PREVIOUS_OUTPUT_BEAT_EXTRACTION, VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
    VIDEO_PREVIOUS_OUTPUT_SHOT_ANALYSIS, VideoContextManifestInput, VideoContextPreviousOutput,
    build_final_video_context_manifest, build_video_context_manifest,
};
#[cfg(test)]
pub(super) use self::project_context::{video_context_character_names, video_profile_context};
pub(super) use self::public_context::final_video_prompt_public_context;

pub(super) fn video_source_metadata(source: &ChapterVideoSourceContext<'_>) -> Value {
    source_metadata_with_flags(
        ChapterSourceContext {
            content: source.content,
        },
        [
            (
                "summary_present",
                source.summary.map(str::trim).is_some_and(|s| !s.is_empty()),
            ),
            ("analysis_present", source.analysis.is_some()),
        ],
    )
}

pub(super) fn validate_prompt_options(
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
    let prompt_output_language =
        PromptOutputLanguage::from_optional_code(options.prompt_output_language.as_deref())?;
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
    let aspect_ratio = options.aspect_ratio.trim();
    if aspect_ratio.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 aspect_ratio 不能为空".to_string(),
        ));
    }
    validate_video_aspect_ratio(aspect_ratio)?;
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
    Ok(ChapterVideoPromptOptions {
        style: style.to_string(),
        prompt_output_language: Some(prompt_output_language.as_str().to_string()),
        prompt_batch_size: options.prompt_batch_size,
        duration_secs: options.duration_secs,
        duration_secs_options,
        aspect_ratio: aspect_ratio.to_string(),
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

fn validate_video_aspect_ratio(aspect_ratio: &str) -> Result<(), AppError> {
    if SUPPORTED_VIDEO_ASPECT_RATIOS.contains(&aspect_ratio) {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "视频提示词生成 aspect_ratio 无效: {aspect_ratio}，必须是 {}",
        SUPPORTED_VIDEO_ASPECT_RATIOS.join("、")
    )))
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

pub(super) fn validate_prompt_duration(duration_secs: i32) -> Result<(), AppError> {
    if !(1..=60).contains(&duration_secs) {
        return Err(AppError::InvalidArgument(
            "视频提示词 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> ChapterVideoPromptOptions {
        ChapterVideoPromptOptions {
            style: " 电影感写实 ".to_string(),
            prompt_output_language: None,
            prompt_batch_size: 2,
            duration_secs: 8,
            duration_secs_options: vec![4, 8],
            aspect_ratio: " 16:9 ".to_string(),
            reference_image_ids: vec![6, 1, 2],
            video_library_image_ids: vec![9, 4],
            duration_capability: None,
            regenerate_existing: false,
            workflow_run_id: None,
        }
    }

    #[test]
    fn validate_prompt_options_preserves_reference_id_order() {
        let validated = validate_prompt_options(options()).expect("valid options");

        assert_eq!(validated.reference_image_ids, vec![6, 1, 2]);
        assert_eq!(validated.video_library_image_ids, vec![9, 4]);
    }

    #[test]
    fn validate_prompt_options_normalizes_duration_options() {
        let mut input = options();
        input.duration_secs = 8;
        input.duration_secs_options = vec![10, 4, 10];

        let validated = validate_prompt_options(input).expect("valid options");

        assert_eq!(validated.duration_secs_options, vec![4, 8, 10]);
    }

    #[test]
    fn validate_prompt_options_allows_large_batch_size_but_rejects_zero() {
        let mut input = options();
        input.prompt_batch_size = 24;

        let validated = validate_prompt_options(input).expect("batch size follows caller context");

        assert_eq!(validated.prompt_batch_size, 24);

        let mut invalid = options();
        invalid.prompt_batch_size = 0;

        let err = validate_prompt_options(invalid).expect_err("zero batch size is invalid");

        assert!(err.to_string().contains("prompt_batch_size 必须大于等于 1"));
    }

    #[test]
    fn validate_prompt_options_rejects_duplicate_reference_ids() {
        let mut input = options();
        input.reference_image_ids = vec![6, 1, 6];

        let err = validate_prompt_options(input).expect_err("duplicate ids are invalid");

        assert!(
            err.to_string()
                .contains("reference_image_ids 包含重复 ID：6")
        );
    }

    #[test]
    fn validate_prompt_options_rejects_duplicate_video_library_ids() {
        let mut input = options();
        input.video_library_image_ids = vec![9, 4, 9];

        let err = validate_prompt_options(input).expect_err("duplicate ids are invalid");

        assert!(
            err.to_string()
                .contains("video_library_image_ids 包含重复 ID：9")
        );
    }

    #[test]
    fn validate_prompt_options_rejects_unsupported_aspect_ratio() {
        let mut input = options();
        input.aspect_ratio = "2:1".to_string();

        let err = validate_prompt_options(input).expect_err("aspect ratio should be limited");

        assert!(err.to_string().contains("aspect_ratio 无效: 2:1"));
        assert!(err.to_string().contains("21:9、16:9、4:3、1:1、3:4、9:16"));
    }
}
