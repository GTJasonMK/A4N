use a4n_db_models::models::chapter::chapter_video_prompt;

use super::super::super::*;
use super::super::inputs::frame_prompt_chain_input;

impl OrchestratorContext {
    pub(in crate::video) async fn generate_chapter_boundary_frame_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &BOUNDARY_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub(in crate::video) async fn generate_chapter_segment_plan(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_shared_segment_plan(
            SharedSegmentPlanInput {
                service: self,
                stage: input,
            },
            chunks,
            progress,
        )
        .await
    }

    pub(in crate::video) async fn generate_chapter_first_frame_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &FIRST_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub(in crate::video) async fn generate_chapter_storyboard_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &STORYBOARD_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }
}
