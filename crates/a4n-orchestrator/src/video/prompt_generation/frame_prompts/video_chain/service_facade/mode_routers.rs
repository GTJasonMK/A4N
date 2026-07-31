//! 按 workflow_mode 分发的 generate/preview 入口，供 runtime 窄 facade 使用。

use a4n_core::models::chapter::chapter_video_prompt;

use super::super::super::{
    AppError, ChapterVideoPromptStageInput, OrchestratorContext, VideoPromptChunk,
    VideoPromptProgress,
};

const MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
const MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
const MODE_CHAT_VIDEO: &str = "chat_video";
const MODE_STORYBOARD: &str = "storyboard";

fn unsupported_workflow_mode(kind: &str, mode: &str) -> AppError {
    AppError::InvalidArgument(format!("不支持的{kind} workflow_mode: {mode}"))
}

impl OrchestratorContext {
    /// 按 stage.workflow_mode 分发章节视频提示词生成。
    pub async fn generate_chapter_video_prompts_for_mode(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        match input.workflow_mode {
            MODE_BOUNDARY_FRAMES => {
                self.generate_chapter_video_prompts(input, chunks, progress)
                    .await
            }
            MODE_FIRST_FRAME_CHAIN => {
                self.generate_chapter_first_frame_video_prompts(input, chunks, progress)
                    .await
            }
            MODE_CHAT_VIDEO => {
                self.generate_chapter_chat_video_prompts(input, chunks, progress)
                    .await
            }
            MODE_STORYBOARD => {
                self.generate_chapter_storyboard_video_prompts(input, chunks, progress)
                    .await
            }
            other => Err(unsupported_workflow_mode("视频提示词生成", other)),
        }
    }

    /// 按 stage.workflow_mode 分发章节帧提示词生成。
    pub async fn generate_chapter_frame_prompts_for_mode(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        match input.workflow_mode {
            MODE_BOUNDARY_FRAMES => {
                self.generate_chapter_boundary_frame_prompts(input, chunks, progress)
                    .await
            }
            MODE_FIRST_FRAME_CHAIN => {
                self.generate_chapter_first_frame_prompts(input, chunks, progress)
                    .await
            }
            MODE_CHAT_VIDEO => Err(AppError::InvalidArgument(
                "Chat 方案不生成帧提示词，请直接生成当前片段的最终提示词".to_string(),
            )),
            MODE_STORYBOARD => {
                self.generate_chapter_storyboard_prompts(input, chunks, progress)
                    .await
            }
            other => Err(unsupported_workflow_mode("帧提示词生成", other)),
        }
    }
}
