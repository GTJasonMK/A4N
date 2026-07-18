use a4n_core::models::chapter::chapter_video_prompt;

use super::super::super::{
    AppError, BOUNDARY_FRAME_PROMPT_GENERATION, CHAT_VIDEO_FRAME_PROMPT_GENERATION,
    ChapterVideoPromptStageInput, FIRST_FRAME_PROMPT_GENERATION, OrchestratorContext,
    STORYBOARD_PROMPT_GENERATION, VideoPromptChunk, VideoPromptProgress,
};
use super::super::{generate_chapter_video_prompt_chain, inputs::video_prompt_chain_input};

impl OrchestratorContext {
    pub async fn generate_chapter_chat_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &CHAT_VIDEO_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &BOUNDARY_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_first_frame_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &FIRST_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_storyboard_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &STORYBOARD_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }
}
