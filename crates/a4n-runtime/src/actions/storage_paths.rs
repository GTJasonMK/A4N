mod orphans;
mod paths;
mod scanning;

pub use a4n_api_contract::storage::{
    OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary, StoragePathsSummary,
};
pub use orphans::{cleanup_orphan_project_data_dirs, list_orphan_project_data_dirs};
pub(crate) use orphans::{
    recover_orphan_project_data_dirs, recover_unreferenced_managed_media_files,
};
pub use paths::*;

#[cfg(test)]
mod tests;
