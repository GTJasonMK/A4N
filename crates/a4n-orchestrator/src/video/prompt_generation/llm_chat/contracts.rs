use std::collections::HashMap;
use std::time::Duration;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_error::AppError;
use a4n_llm::{
    provider::{ChatRequest, UsageInfo},
    trace::llm_usage_summary_text,
};

use a4n_llm::context::LlmContextManifest;
use a4n_progress::{ProgressSpan, StructuredJsonProgressSpec};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::{VideoPromptChunk, VideoPromptProgress, VideoPromptProgressUpdate};
use crate::OrchestratorContext;

#[derive(Clone, Copy)]
pub(in crate::video::prompt_generation) struct VideoPromptStepProgress<'a> {
    pub(in crate::video::prompt_generation) current: u32,
    pub(in crate::video::prompt_generation) total: u32,
    pub(in crate::video::prompt_generation) message: &'a str,
    pub(in crate::video::prompt_generation) span: Option<ProgressSpan>,
}

#[derive(Debug, Clone)]
pub(in crate::video::prompt_generation) struct RenderedVideoPromptTrace {
    pub(in crate::video::prompt_generation) prompt_name: String,
    pub(in crate::video::prompt_generation) request: ChatRequest,
    pub(in crate::video::prompt_generation) response_text: String,
    pub(in crate::video::prompt_generation) usage: Option<UsageInfo>,
    pub(in crate::video::prompt_generation) finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::video::prompt_generation) struct VideoPromptLlmCallTraceContext {
    pub(in crate::video::prompt_generation) workflow_run_id: i64,
    pub(in crate::video::prompt_generation) call_key: String,
    pub(in crate::video::prompt_generation) input_fingerprint_sha256: String,
}

pub(in crate::video::prompt_generation) struct VideoPromptRenderInput<'a, 'step, 'progress, 'chunk>
{
    pub(in crate::video::prompt_generation) service: &'a OrchestratorContext,
    pub(in crate::video::prompt_generation) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::video::prompt_generation) prompt_name: &'a str,
    pub(in crate::video::prompt_generation) project_id: i64,
    pub(in crate::video::prompt_generation) variables: &'a HashMap<String, Value>,
    pub(in crate::video::prompt_generation) context_manifest: LlmContextManifest,
    pub(in crate::video::prompt_generation) temperature: f32,
    pub(in crate::video::prompt_generation) llm_call_trace: VideoPromptLlmCallTraceContext,
    pub(in crate::video::prompt_generation) cancel_token: Option<&'a CancellationToken>,
    pub(in crate::video::prompt_generation) step: VideoPromptStepProgress<'step>,
    pub(in crate::video::prompt_generation) progress:
        &'a mut Option<VideoPromptProgress<'progress>>,
    pub(in crate::video::prompt_generation) chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
    pub(in crate::video::prompt_generation) structured_progress: Option<StructuredJsonProgressSpec>,
}

impl<'a> VideoPromptStepProgress<'a> {
    pub(in crate::video::prompt_generation) fn new(
        current: u32,
        total: u32,
        message: &'a str,
    ) -> Self {
        Self {
            current,
            total,
            message,
            span: None,
        }
    }

    pub(in crate::video::prompt_generation) fn with_span(mut self, span: ProgressSpan) -> Self {
        self.span = Some(span);
        self
    }
}

pub(in crate::video::prompt_generation) fn report_video_prompt_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress.as_deref_mut() {
        report(VideoPromptProgressUpdate::progress(current, total, message))?;
    }
    Ok(())
}

pub(in crate::video::prompt_generation) fn report_video_prompt_item_committed(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    item_kind: &str,
    item_number: i32,
    item_position: usize,
    item_total: usize,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress.as_deref_mut() {
        report(VideoPromptProgressUpdate::committed(
            current,
            total,
            item_kind,
            item_number,
            item_position,
            item_total,
            message,
        ))?;
    }
    Ok(())
}

pub(in crate::video::prompt_generation) fn report_video_prompt_checkpoint(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    checkpoint_key: impl Into<String>,
    item_index: usize,
    item_total: usize,
    draft_text: impl Into<String>,
    message: impl Into<String>,
) -> Result<(), AppError> {
    if let Some(report) = progress.as_deref_mut() {
        report(VideoPromptProgressUpdate::checkpoint(
            current,
            total,
            checkpoint_key,
            item_index,
            item_total,
            draft_text,
            message,
        ))?;
    }
    Ok(())
}

pub(super) fn report_video_prompt_waiting(
    progress: &mut Option<VideoPromptProgress<'_>>,
    step: VideoPromptStepProgress<'_>,
    current: u32,
    elapsed: Duration,
    phase: &str,
) -> Result<(), AppError> {
    let elapsed_secs = elapsed.as_secs().max(1);
    let message = format!("{phase}：{}（已等待 {} 秒）", step.message, elapsed_secs);
    report_video_prompt_progress(progress, current, step.total, &message)
}

pub(super) fn report_video_prompt_usage(
    progress: &mut Option<VideoPromptProgress<'_>>,
    step: VideoPromptStepProgress<'_>,
    usage: Option<&UsageInfo>,
) -> Result<(), AppError> {
    let message = format!(
        "模型请求完成：{}；{}",
        step.message,
        llm_usage_summary_text(usage)
    );
    report_video_prompt_progress(progress, step.current, step.total, &message)
}

#[cfg(test)]
mod tests {
    use super::{
        AppError, Duration, VideoPromptProgress, VideoPromptProgressUpdate,
        VideoPromptStepProgress, report_video_prompt_item_committed, report_video_prompt_waiting,
    };

    #[test]
    fn reports_waiting_progress_at_current_floor() {
        let mut calls = Vec::<(u32, u32, String)>::new();
        {
            let mut progress_fn = |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
                calls.push((update.current, update.total, update.message));
                Ok(())
            };
            let mut progress: Option<VideoPromptProgress<'_>> =
                Some(&mut progress_fn as VideoPromptProgress<'_>);

            report_video_prompt_waiting(
                &mut progress,
                VideoPromptStepProgress::new(20, 100, "生成中"),
                57,
                Duration::from_secs(9),
                "等待模型继续输出",
            )
            .expect("waiting progress should report");
        }

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 57);
        assert_eq!(calls[0].1, 100);
        assert!(calls[0].2.contains("等待模型继续输出：生成中"));
    }

    #[test]
    fn committed_progress_has_stable_item_identity() {
        let mut calls = Vec::new();
        {
            let mut progress_fn = |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
                calls.push(update);
                Ok(())
            };
            let mut progress: Option<VideoPromptProgress<'_>> = Some(&mut progress_fn);

            report_video_prompt_item_committed(
                &mut progress,
                75,
                100,
                "storyboard_prompt",
                2,
                2,
                4,
                "第 2 条已保存",
            )
            .expect("committed progress should report");
        }

        let update = calls.pop().expect("one update");
        assert_eq!(update.stage_code, "video_prompt_item_committed");
        assert_eq!(update.item_key.as_deref(), Some("storyboard_prompt:2"));
        assert_eq!(update.item_index, Some(1));
        assert_eq!(update.item_total, Some(4));
        assert_eq!(
            update.phase_kind,
            a4n_domain::progress::ProgressPhaseKind::Save
        );

        let single =
            VideoPromptProgressUpdate::committed(100, 100, "frame_segment_plan", 0, 1, 1, "已保存");
        assert_eq!(single.item_index, Some(0));
    }
}
