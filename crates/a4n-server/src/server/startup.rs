mod configuration;
mod foundation;
mod lifecycle;
mod remaining;
mod state;

use configuration::load;
use foundation::start as start_foundation;
use lifecycle::run as run_lifecycle;
use remaining::start as start_remaining;

pub(crate) fn run() -> anyhow::Result<()> {
    #[cfg(feature = "test-embedded-processes")]
    if super::embedded::run_if_requested()?.is_some() {
        return Ok(());
    }
    let config = load()?;
    let foundation = start_foundation(&config)?;
    let runtime = start_remaining(config, foundation)?;
    run_lifecycle(runtime)
}
