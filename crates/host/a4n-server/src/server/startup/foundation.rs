use super::super::*;
use super::state::StartupConfig;

pub(crate) fn start(config: &StartupConfig) -> anyhow::Result<WorkerProcessRegistry> {
    start_managed_workers(&config.workers)
}
