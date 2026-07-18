use super::super::{
    GenerateChapterVideoPromptInput, ModelRequestPreview, VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN, VIDEO_WORKFLOW_MODE_STORYBOARD,
    VideoPromptRequestPreview, annotate_preview_placeholder_inputs,
    prepare_chapter_video_prompt_preview, replan_video_prompt_request_preview_for_state,
    source_context, video_prompt_preview_to_model_request_preview,
};
use crate::AppState;

pub async fn preview_chapter_video_frame_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let prepared = prepare_chapter_video_prompt_preview(
        state,
        input,
        "预览边界帧提示词",
        "预览帧提示词",
        "Chat 生视频模式使用外部 Chat 视频模型，帧提示词预览不套用本地视频模型固定时长档位",
    )
    .await?;
    let source_context = source_context(&prepared.source);
    let preview = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .orchestrator
            .preview_chapter_chat_video_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .orchestrator
            .preview_chapter_first_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .orchestrator
            .preview_chapter_storyboard_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else {
        state
            .orchestrator
            .preview_chapter_frame_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    }
    .map_err(|error| error.to_string())?;
    replan_video_prompt_request_preview_for_state(
        state,
        annotate_preview_placeholder_inputs(preview, &prepared.placeholder_notes),
    )
    .await
}

pub async fn preview_chapter_video_frame_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_frame_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
}

pub async fn preview_chapter_video_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let prepared = prepare_chapter_video_prompt_preview(
        state,
        input,
        "预览视频提示词",
        "预览视频提示词",
        "Chat 生视频模式使用外部 Chat 视频模型，视频提示词预览不套用本地视频模型固定时长档位",
    )
    .await?;
    let source_context = source_context(&prepared.source);
    let preview = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .orchestrator
            .preview_chapter_chat_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .orchestrator
            .preview_chapter_first_frame_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .orchestrator
            .preview_chapter_storyboard_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    } else {
        state
            .orchestrator
            .preview_chapter_video_prompt_requests(
                prepared.project_id,
                prepared.chapter_id,
                source_context,
                state,
                &prepared.vars,
                prepared.prompt_options,
            )
            .await
    }
    .map_err(|error| error.to_string())?;
    replan_video_prompt_request_preview_for_state(
        state,
        annotate_preview_placeholder_inputs(preview, &prepared.placeholder_notes),
    )
    .await
}

pub async fn preview_chapter_video_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
}
