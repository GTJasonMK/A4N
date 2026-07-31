use a4n_core::error::AppError;

use crate::video::OrchestratorContext;

use super::VideoPromptRequestPreview;
use super::inputs::LocalPromptPreviewInput;
use super::spec::LocalVideoPromptPreviewSpec;
use super::video_completion::CompleteVideoPromptPreviewInput;
use super::video_predecessors::ResolveVideoPromptPredecessorsInput;

impl OrchestratorContext {
    pub(super) async fn preview_local_video_prompt_requests(
        &self,
        request: LocalPromptPreviewInput<'_, '_>,
        spec: LocalVideoPromptPreviewSpec,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        let LocalPromptPreviewInput {
            project_id,
            chapter_id,
            source,
            prompt_renderer,
            variables,
            options,
        } = request;
        let prepared = self
            .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, false)
            .await?;
        let mut steps = Vec::new();
        let predecessors = self
            .resolve_video_prompt_predecessors(
                &mut steps,
                ResolveVideoPromptPredecessorsInput {
                    prompt_renderer,
                    project_id,
                    chapter_id,
                    source: &source,
                    prepared: &prepared,
                    spec,
                },
            )
            .await?;
        self.complete_video_prompt_preview(
            steps,
            CompleteVideoPromptPreviewInput {
                prompt_renderer,
                project_id,
                chapter_id,
                source: &source,
                prepared: &prepared,
                spec,
                predecessors,
            },
        )
        .await
    }
}
