use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::Value;

use super::super::*;

impl OrchestratorContext {
    pub async fn preview_chapter_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_renderer,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_chat_video_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_renderer,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::chat_video_boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_storyboard_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_renderer,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::storyboard(),
        )
        .await
    }

    pub async fn preview_chapter_first_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_renderer,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::first_frame_chain(),
        )
        .await
    }
}
