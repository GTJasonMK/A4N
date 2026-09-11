use a4n_db_models::models::source_sync::source_sync_run;

#[derive(Debug, Clone)]
pub struct FinalizeSourceSyncApplyRunInput {
    pub run: source_sync_run::Model,
    pub detach_requested: bool,
}
