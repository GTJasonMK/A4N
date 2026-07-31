use std::collections::BTreeSet;

use a4n_core::{
    error::AppError,
    models::{chapter::chapter, project::text_anchor},
    structured_progress::{FieldMilestone, ProgressSpan},
};
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::asset::ObjectProfileDraft;

pub(super) const OBJECT_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
pub(super) const OBJECT_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(10, 75);
pub(super) const OBJECT_DISCOVERY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("name", "物体名称", 1),
    FieldMilestone::new("aliases", "别名", 1),
    FieldMilestone::new("object_type", "物体类型", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("visual_description", "外观描述", 1),
    FieldMilestone::new("material_and_state", "材质状态", 1),
    FieldMilestone::new("story_usage", "叙事用途", 1),
    FieldMilestone::new("visual_usage", "视觉用途", 1),
    FieldMilestone::new("generation_usage", "生成用途", 1),
    FieldMilestone::new("location_context", "位置关系", 1),
    FieldMilestone::new("evidence_anchor_ids", "证据锚点", 1),
];

pub type ObjectDiscoveryProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub struct ObjectDiscoveryProgress<'a> {
    pub reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub active_task_id: Option<&'a str>,
}

impl ObjectDiscoveryProgress<'_> {
    pub(super) fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, OBJECT_DISCOVERY_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDiscoveryInput {
    pub project_id: i64,
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ObjectDiscoveryRequestPreview {
    pub analyzed_chapter_count: usize,
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RequiredObjectDiscoveryInput {
    pub(super) project_id: i64,
    pub(super) chapter_number: Option<i32>,
}

pub(super) struct ObjectDiscoveryRequestContext {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) request: ChatRequest,
}

#[derive(Debug)]
pub(super) struct ParsedObjectProfileDraft {
    pub(super) draft: ObjectProfileDraft,
    pub(super) evidence_anchor_ids: Vec<i64>,
}

pub(super) struct ObjectDiscoveryCallResult {
    pub(super) parsed_drafts: Vec<ParsedObjectProfileDraft>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}

pub(super) struct ObjectDiscoveryCallCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_output: Value,
}

pub(super) struct PrepareObjectDiscoveryRequestInput<'a> {
    pub(super) input: ObjectDiscoveryInput,
    pub(super) anchor_mode: ChapterAnchorMode,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
}
