use super::ObjectDiscoveryProgressReporter;
use a4n_core::structured_progress::{FieldMilestone, ProgressSpan};
use tokio_util::sync::CancellationToken;

pub(super) const OBJECT_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
pub(super) const OBJECT_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(10, 75);
pub(super) const OBJECT_DISCOVERY_CALL_KEY: &str = "object_discovery.llm";
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

pub(crate) struct ObjectDiscoveryProgress<'a> {
    reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ObjectDiscoveryProgress<'a> {
    pub(crate) fn new(
        reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            Err(message.to_string())
        } else {
            Ok(())
        }
    }
}

pub(super) fn report_object_discovery_progress(
    progress: Option<&ObjectDiscoveryProgress<'_>>,
    current: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.progress(current, OBJECT_DISCOVERY_PROGRESS_TOTAL, message);
        progress.check_cancelled(cancelled_message)?;
    }
    Ok(())
}
