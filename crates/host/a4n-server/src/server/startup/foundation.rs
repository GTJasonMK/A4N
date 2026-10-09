use super::super::*;
use super::state::StartupConfig;

pub(crate) fn start(config: &mut StartupConfig) -> anyhow::Result<WorkerProcessRegistry> {
    let reserved_ports = config
        .workers
        .iter()
        .filter(|worker| worker.address_is_explicit)
        .map(|worker| worker.address.port())
        .chain([config.web_addr.port(), config.control_addr.port()])
        .collect::<Vec<_>>();
    start_managed_workers(&mut config.workers, &reserved_ports)
}
