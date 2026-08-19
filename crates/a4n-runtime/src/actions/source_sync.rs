mod application;
mod contracts;
mod detach;
mod preview;
mod projections;

pub use application::apply_source_sync_decisions;
pub use contracts::{
    ApplySourceSyncInput, SourceSyncConflictSummary, SourceSyncDecisionInput,
    SourceSyncPlanSummary, SourceSyncProjectInput, SourceSyncRunSummary,
};
pub use detach::detach_derived_project;
pub use preview::preview_source_sync;

#[cfg(test)]
mod tests;
