use std::collections::HashMap;

use serde_json::Value;

use a4n_core::error::AppError;

use crate::video::{ChapterVideoPromptOptions, ChapterVideoSourceContext, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::{LocalPromptPreviewInput, LocalVideoPromptPreviewSpec, VideoPromptRequestPreview};

impl OrchestratorContext {
    pub async fn preview_chapter_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
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

    pub async fn preview_chapter_storyboard_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
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

    pub async fn preview_chapter_chat_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
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

    pub async fn preview_chapter_first_frame_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
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
