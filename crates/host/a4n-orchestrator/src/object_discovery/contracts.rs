use std::collections::BTreeSet;

use a4n_api_contract::profile::ObjectProfileInput;
use a4n_domain::{chapter::ChapterRecord, text_anchor::TextAnchorRecord};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::{FieldMilestone, ProgressSpan};
use tokio_util::sync::CancellationToken;

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

pub(crate) type ObjectDiscoveryProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub(crate) struct ObjectDiscoveryProgress<'a> {
    pub(crate) reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
    /// 活体信号出口：与字段级里程碑取自同一个 token 流。
    pub(crate) stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub(crate) cancel_token: Option<&'a CancellationToken>,
    pub(crate) active_task_id: Option<&'a str>,
}

impl ObjectDiscoveryProgress<'_> {
    pub(super) fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, super::OBJECT_DISCOVERY_PROGRESS_TOTAL, message)?;
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
pub(crate) struct ObjectDiscoveryInput {
    pub(crate) project_id: i64,
    pub(crate) chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) struct ObjectDiscoveryRequestContext {
    pub(super) chapters: Vec<ChapterRecord>,
    pub(super) text_anchors: Vec<TextAnchorRecord>,
}

#[derive(Debug)]
pub(super) struct ParsedObjectProfileDraft {
    pub(super) draft: ObjectProfileInput,
    pub(super) evidence_anchor_ids: Vec<i64>,
}

pub(super) struct ObjectDiscoveryCallResult {
    pub(super) parsed_drafts: Vec<ParsedObjectProfileDraft>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}
