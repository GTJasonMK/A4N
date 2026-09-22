mod contracts;
mod matching;
mod records {
    mod mutations;
    mod reuse;
}

#[cfg(test)]
pub use contracts::VideoPromptUpdateInput;
pub(in crate::video) use contracts::{PromptWorkflowRowsContext, VideoExecutionIdentityMatch};
pub(in crate::video) use matching::{
    expected_contract_version, reusable_rows_are_complete,
    video_execution_segment_identity_matches, video_execution_segment_lineage_matches,
    video_generation_asset_fingerprint,
};
