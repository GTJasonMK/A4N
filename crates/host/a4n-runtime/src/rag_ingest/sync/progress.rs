mod reporter;
mod stats;

pub type RagIngestProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub use reporter::RagIngestProgress;
pub(crate) use stats::ExpectedSyncStats;
