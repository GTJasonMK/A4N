//! Story fact feedback persistence; the existing material records are the read models.

mod apply;
mod archive;
mod profiles;
pub use archive::{FactFeedbackArchive, load_feedback_archive, reset_feedback_coverage};
mod records;
mod replay;

pub use apply::{ConsolidationApplyInput, ConsolidationApplyResult, apply_consolidation};
pub use profiles::profile_facts_before_chapter;
pub(crate) use profiles::{
    profile_fact_snapshot, record_profile_fact_observation, restore_profile_fact,
};
pub use records::{
    FactChangeRecordInput, FeedbackSource, current_feedback_sources, feedback_changes,
    feedback_config, feedback_interval, feedback_intervals, record_fact_change,
    record_feedback_completion, source_revision, update_feedback_interval,
};
pub use replay::{
    invalidate_feedback_sources, profile_enrichment_before_chapter, world_setting_before_chapter,
};

#[derive(Debug, thiserror::Error)]
pub enum FactFeedbackDbError {
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Blueprint(#[from] crate::blueprint::BlueprintDbError),
    #[error(transparent)]
    Workflow(#[from] crate::workflow::WorkflowWriteError),
    #[error("{0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests;
