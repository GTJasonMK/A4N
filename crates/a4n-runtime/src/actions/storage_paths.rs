mod orphans;
mod paths;
mod scanning;

pub use a4n_api_contract::storage::{
    OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary, StoragePathsSummary,
};
pub use orphans::{cleanup_orphan_project_data_dirs, list_orphan_project_data_dirs};
pub use paths::{get_storage_paths, log_dir_path};

#[cfg(test)]
mod tests;
