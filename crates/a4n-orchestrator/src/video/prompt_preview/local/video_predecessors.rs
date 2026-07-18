use serde_json::Value;

use crate::video::prompt_json::VideoSegmentPlan;

mod frames;
mod segment;

pub(super) use segment::ResolveVideoPromptPredecessorsInput;

pub(super) enum SegmentPreviewSource {
    Placeholder,
    Reusable { execution_identity: Value },
}

impl SegmentPreviewSource {
    pub(super) fn is_placeholder(&self) -> bool {
        matches!(self, Self::Placeholder)
    }
}

pub(super) struct ResolvedVideoPromptPredecessors {
    pub(super) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) frame_json: Value,
    pub(super) segment_source: SegmentPreviewSource,
    pub(super) frame_is_placeholder: bool,
}
