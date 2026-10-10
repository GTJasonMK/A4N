use super::super::*;
use super::state::Runtime;
use super::supervision::run as supervise;

pub(crate) fn run(runtime: Runtime) -> anyhow::Result<()> {
    let Runtime {
        config,
        mut edge,
        mut control,
        mut workers,
        mut control_fingerprint,
        mut edge_fingerprint,
        mut reload_marker_fingerprint,
    } = runtime;
    let supervision_result = supervise(
        &config,
        &mut edge,
        &mut control,
        &mut workers,
        &mut control_fingerprint,
        &mut edge_fingerprint,
        &mut reload_marker_fingerprint,
    );
    let cleanup_results = stop_managed_workers(&mut workers);
    let mut failures = Vec::new();
    if let Err(error) = supervision_result {
        failures.push(format!("Worker supervision: {error:#}"));
    }
    for (name, result) in [
        ("Edge", stop_child(&mut edge)),
        ("Control", stop_child(&mut control)),
    ] {
        if let Err(error) = result {
            failures.push(format!("{name}: {error:#}"));
        }
    }
    for (name, result) in cleanup_results {
        if let Err(error) = result {
            failures.push(format!("{name}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(failures.join("；"))
    }
}
