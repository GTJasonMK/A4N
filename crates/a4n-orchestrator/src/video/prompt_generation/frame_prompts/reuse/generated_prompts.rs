use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;
use serde_json::Value;

use crate::video::library::ensure_video_reference_ids_known;
use crate::video::prompt_json::{ChatVideoPromptShot, VideoPromptShot, VideoSegmentPlan};
use crate::video::prompt_store::PromptWorkflowMode;

use super::super::{
    FramePromptGenerationSpec, parse_single_chat_video_prompt_shot, parse_single_video_prompt_shot,
    reusable_boundary_video_prompts_from_rows, reusable_chat_video_prompts_from_rows,
    reusable_first_frame_video_prompts_from_rows, validate_chat_video_prompts,
    validate_partial_chat_video_prompts, validate_partial_video_prompts, validate_video_prompts,
};

#[derive(Clone)]
pub(in crate::video::prompt_generation::frame_prompts) enum GeneratedVideoPromptShot {
    Api(Box<VideoPromptShot>),
    Chat(ChatVideoPromptShot),
}

impl GeneratedVideoPromptShot {
    pub(in crate::video::prompt_generation::frame_prompts) fn shot_number(&self) -> i32 {
        match self {
            Self::Api(prompt) => prompt.shot_number,
            Self::Chat(prompt) => prompt.shot_number,
        }
    }

    pub(in crate::video::prompt_generation::frame_prompts) fn to_json_value(
        &self,
    ) -> Result<Value, AppError> {
        match self {
            Self::Api(prompt) => serde_json::to_value(prompt.as_ref()),
            Self::Chat(prompt) => serde_json::to_value(prompt),
        }
        .map_err(|err| AppError::InvalidArgument(format!("视频提示词序列化失败：{err}")))
    }
}

pub(in crate::video::prompt_generation::frame_prompts) fn reusable_generated_video_prompts_from_rows(
    spec: &FramePromptGenerationSpec,
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, GeneratedVideoPromptShot>, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能读取卡片视频提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames => Ok(reusable_boundary_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| {
                (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
            })
            .collect()),
        PromptWorkflowMode::FirstFrameChain => {
            Ok(reusable_first_frame_video_prompts_from_rows(rows)?
                .into_iter()
                .map(|(shot_number, prompt)| {
                    (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
                })
                .collect())
        }
        PromptWorkflowMode::ChatVideo => Ok(reusable_chat_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| (shot_number, GeneratedVideoPromptShot::Chat(prompt)))
            .collect()),
        PromptWorkflowMode::Storyboard => Ok(reusable_boundary_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| {
                (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
            })
            .collect()),
    }
}

pub(in crate::video::prompt_generation::frame_prompts) fn api_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    label: &str,
) -> Result<Vec<VideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|prompt| match prompt {
            GeneratedVideoPromptShot::Api(prompt) => Ok(prompt.as_ref().clone()),
            GeneratedVideoPromptShot::Chat(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 Chat 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(in crate::video::prompt_generation::frame_prompts) fn chat_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    label: &str,
) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|prompt| match prompt {
            GeneratedVideoPromptShot::Chat(prompt) => Ok(prompt.clone()),
            GeneratedVideoPromptShot::Api(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 API 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(in crate::video::prompt_generation::frame_prompts) fn chat_generated_video_prompt_map(
    prompts: &BTreeMap<i32, GeneratedVideoPromptShot>,
    label: &str,
) -> Result<BTreeMap<i32, ChatVideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|(shot_number, prompt)| match prompt {
            GeneratedVideoPromptShot::Chat(prompt) => Ok((*shot_number, prompt.clone())),
            GeneratedVideoPromptShot::Api(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 API 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(in crate::video::prompt_generation::frame_prompts) fn parse_single_generated_video_prompt_shot(
    spec: &FramePromptGenerationSpec,
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    allowed_reference_ids: &HashSet<i64>,
    context: &str,
) -> Result<GeneratedVideoPromptShot, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能解析卡片视频提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => parse_single_video_prompt_shot(
            raw,
            segment_by_number,
            prompt_output_language,
            expected_shot_number,
            context,
        )
        .map(|prompt| GeneratedVideoPromptShot::Api(Box::new(prompt))),
        PromptWorkflowMode::ChatVideo => {
            let prompt = parse_single_chat_video_prompt_shot(
                raw,
                segment_by_number,
                prompt_output_language,
                expected_shot_number,
                context,
            )?;
            ensure_video_reference_ids_known(
                &prompt.reference_image_ids,
                allowed_reference_ids,
                &format!("{} #{}", spec.video_prompt_label, prompt.shot_number),
            )?;
            Ok(GeneratedVideoPromptShot::Chat(prompt))
        }
    }
}

pub(in crate::video::prompt_generation::frame_prompts) fn validate_generated_video_prompts(
    spec: &FramePromptGenerationSpec,
    prompts: &[GeneratedVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能校验卡片视频提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => validate_video_prompts(
            &api_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
        PromptWorkflowMode::ChatVideo => validate_chat_video_prompts(
            &chat_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
    }
}

pub(in crate::video::prompt_generation::frame_prompts) fn validate_partial_generated_video_prompts(
    spec: &FramePromptGenerationSpec,
    prompts: &[GeneratedVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能校验卡片视频提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => validate_partial_video_prompts(
            &api_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
        PromptWorkflowMode::ChatVideo => validate_partial_chat_video_prompts(
            &chat_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
    }
}
