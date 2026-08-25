mod application;
mod detach;
mod preview;
mod projections;

pub use a4n_api_contract::source_sync::{
    ApplySourceSyncInput, SourceSyncConflictSummary, SourceSyncPlanSummary, SourceSyncProjectInput,
    SourceSyncRunSummary,
};
pub use application::apply_source_sync_decisions;
pub use detach::detach_derived_project;
pub use preview::preview_source_sync;

#[cfg(test)]
mod tests;
