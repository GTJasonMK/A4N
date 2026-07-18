use a4n_core::structured_progress::StructuredJsonProgressSpec;
use a4n_llm::provider::{ChatRequest, UsageInfo};
use tokio_util::sync::CancellationToken;

use super::super::super::super::{VideoPromptChunk, VideoPromptProgress};
use super::super::contracts::VideoPromptStepProgress;
use crate::OrchestratorContext;

pub(in crate::video::prompt_generation::llm_chat) struct StreamedVideoPromptResult {
    pub(in crate::video::prompt_generation::llm_chat) content: String,
    pub(in crate::video::prompt_generation::llm_chat) usage: Option<UsageInfo>,
    pub(in crate::video::prompt_generation::llm_chat) finish_reason: Option<String>,
}

pub(in crate::video::prompt_generation::llm_chat) struct StreamVideoPromptInput<
    'a,
    'step,
    'progress,
    'chunk,
> {
    pub(in crate::video::prompt_generation::llm_chat) service: &'a OrchestratorContext,
    pub(in crate::video::prompt_generation::llm_chat) request: ChatRequest,
    pub(in crate::video::prompt_generation::llm_chat) cancel_token: Option<&'a CancellationToken>,
    pub(in crate::video::prompt_generation::llm_chat) step: VideoPromptStepProgress<'step>,
    pub(in crate::video::prompt_generation::llm_chat) progress:
        &'a mut Option<VideoPromptProgress<'progress>>,
    pub(in crate::video::prompt_generation::llm_chat) chunks:
        &'a mut Option<VideoPromptChunk<'chunk>>,
    pub(in crate::video::prompt_generation::llm_chat) prompt_name: &'a str,
    pub(in crate::video::prompt_generation::llm_chat) structured_progress:
        Option<StructuredJsonProgressSpec>,
}
