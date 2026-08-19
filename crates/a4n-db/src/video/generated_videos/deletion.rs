mod locking;
mod missing;
mod normal;
mod references;

pub use missing::delete_missing_generated_video_references_record;
pub use normal::delete_generated_video_record_with_cleanup;
pub(crate) use references::clear_generated_video_references_record;
