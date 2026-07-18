mod context;
mod evidence;
mod execution;
mod parsing;
mod preview;
mod progress;
mod result;
mod types;
mod workflow;

pub(crate) use execution::discover_object_profiles_for_chapters;
pub(crate) use preview::object_discovery_preview_steps;
pub(crate) use progress::ObjectDiscoveryProgress;
pub(crate) use types::ObjectDiscoveryPreviewStep;

pub(crate) type ObjectDiscoveryProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

#[cfg(test)]
mod tests;
