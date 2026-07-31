use a4n_prompt::rendering::PromptRenderer;

use crate::video::{ChapterVideoSourceContext, OrchestratorContext};

use super::super::super::inputs::PreparedVideoPromptPreviewInputs;
use super::super::super::spec::{LocalFramePromptPreviewMode, LocalVideoPromptPreviewSpec};

#[derive(Clone, Copy)]
pub(super) struct FrameWorkflowContext<'a, 'source> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: LocalVideoPromptPreviewSpec,
    pub(super) mode: LocalFramePromptPreviewMode,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) input: &'a PreparedVideoPromptPreviewInputs,
}
