use super::super::*;
use super::state::StartupConfig;

pub(crate) fn load() -> anyhow::Result<StartupConfig> {
    let control_addr = control_addr()?;
    let workers = catalog_worker_specs(control_addr)?;
    let control_binary = control_binary()?;
    let edge_binary = edge_binary()?;
    let reload_marker = reload_marker_path();
    Ok(StartupConfig {
        control_addr,
        workers,
        control_fingerprint: binary_fingerprint(&control_binary)?,
        edge_fingerprint: binary_fingerprint(&edge_binary)?,
        reload_marker_fingerprint: optional_binary_fingerprint(reload_marker.as_deref())?,
        control_binary,
        edge_binary,
        reload_marker,
    })
}
