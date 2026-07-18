use super::super::{
    GeneratePromptOptionsInput, MAX_CHAT_REFERENCE_IMAGE_IDS, MAX_NON_CHAT_REFERENCE_IMAGE_IDS,
    VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES, VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN, VIDEO_WORKFLOW_MODE_STORYBOARD,
};
use crate::AppState;
use a4n_core::{
    error::AppError, models::chapter::chapter_video_prompt, prompt_language::PromptOutputLanguage,
};
use a4n_orchestrator::{
    video::{ChapterVideoPromptOptions, validate_prompt_options_with_reference_limit},
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED},
};

pub(in crate::actions::video_prompts) async fn finalize_video_prompt_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<Vec<chapter_video_prompt::Model>, AppError>,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    match result {
        Ok(rows) => {
            state
                .orchestrator
                .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(rows)
        }
        Err(err) => {
            let text = err.to_string();
            state
                .orchestrator
                .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_FAILED, true, Some(&text))
                .await
                .map_err(|e| format!("{text}; 同步视频工作流状态失败: {e}"))?;
            Err(text)
        }
    }
}

pub(in crate::actions::video_prompts) fn validate_generate_prompt_options_inner(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
    blank_style_placeholder: Option<&str>,
) -> Result<ChapterVideoPromptOptions, String> {
    let mut style = input.style.trim().to_string();
    if style.is_empty() {
        style = blank_style_placeholder
            .ok_or_else(|| "视频提示词生成缺少 style".to_string())?
            .to_string();
    }
    let limit = if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        MAX_CHAT_REFERENCE_IMAGE_IDS
    } else {
        MAX_NON_CHAT_REFERENCE_IMAGE_IDS
    };
    let prompt_output_language =
        PromptOutputLanguage::from_optional_code(input.prompt_output_language.as_deref())
            .map_err(|error| error.to_string())?;
    validate_prompt_options_with_reference_limit(
        ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            prompt_batch_size: input.prompt_batch_size,
            duration_secs: input.duration_secs,
            duration_secs_options: input.duration_secs_options,
            aspect_ratio: input.aspect_ratio,
            reference_image_ids: input.reference_image_ids,
            video_library_image_ids: input.video_library_image_ids,
            duration_capability: None,
            regenerate_existing: input.regenerate_existing,
            workflow_run_id: None,
        },
        limit,
    )
    .map_err(|e| e.to_string())
}

pub(in crate::actions::video_prompts) fn ensure_video_workflow_mode(
    input: &GeneratePromptOptionsInput,
    allowed: &[&str],
    operation: &str,
) -> Result<String, String> {
    let mode = normalize_video_workflow_mode(input.workflow_mode.as_deref())?;
    if !allowed.contains(&mode.as_str()) {
        return Err(format!("{operation} 不支持当前视频生成模式：{mode}"));
    }
    Ok(mode)
}
pub(in crate::actions::video_prompts) fn normalize_video_workflow_mode(
    mode: Option<&str>,
) -> Result<String, String> {
    let Some(mode) = mode.map(str::trim).filter(|m| !m.is_empty()) else {
        return Err("视频提示词生成缺少 workflow_mode".to_string());
    };
    match mode {
        VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        | VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
        | VIDEO_WORKFLOW_MODE_CHAT_VIDEO
        | VIDEO_WORKFLOW_MODE_STORYBOARD => Ok(mode.to_string()),
        other => Err(format!("视频提示词生成 workflow_mode 无效: {other}")),
    }
}
