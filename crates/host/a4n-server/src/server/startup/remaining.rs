use super::super::*;
use super::state::{Runtime, StartupConfig};

pub(crate) fn start(
    config: StartupConfig,
    mut workers: WorkerProcessRegistry,
) -> anyhow::Result<Runtime> {
    let mut control = match spawn_control_from_workers(
        &config.control_binary,
        config.control_addr,
        &config.workers,
    ) {
        Ok(child) => child,
        Err(error) => {
            return Err(with_cleanup_errors(
                error,
                stop_managed_workers(&mut workers),
            ));
        }
    };
    if let Err(error) = wait_for_control(&mut control, config.control_addr) {
        let cleanup: Vec<_> = stop_child(&mut control)
            .err()
            .into_iter()
            .map(|error| ("control".to_string(), Err(error)))
            .chain(stop_managed_workers(&mut workers))
            .collect();
        return Err(with_cleanup_errors(error, cleanup));
    }
    let edge = match spawn_edge(&config.edge_binary, config.control_addr) {
        Ok(child) => child,
        Err(error) => {
            let cleanup: Vec<_> = stop_child(&mut control)
                .err()
                .into_iter()
                .map(|error| ("control".to_string(), Err(error)))
                .chain(stop_managed_workers(&mut workers))
                .collect();
            return Err(with_cleanup_errors(error, cleanup));
        }
    };
    Ok(Runtime {
        control_fingerprint: config.control_fingerprint,
        edge_fingerprint: config.edge_fingerprint,
        reload_marker_fingerprint: config.reload_marker_fingerprint,
        config,
        edge,
        control,
        workers,
    })
}

fn with_cleanup_errors(
    error: anyhow::Error,
    cleanup: impl IntoIterator<Item = (String, anyhow::Result<()>)>,
) -> anyhow::Error {
    let failures = cleanup
        .into_iter()
        .filter_map(|(name, result)| result.err().map(|error| format!("{name}: {error:#}")))
        .collect::<Vec<_>>();
    if failures.is_empty() {
        error
    } else {
        anyhow::anyhow!("{error:#}; 启动清理失败: {}", failures.join("; "))
    }
}
