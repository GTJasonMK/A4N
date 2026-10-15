mod locking;
mod missing;
mod normal;
mod references;

pub use missing::{
    delete_missing_generated_video_references_record,
    delete_missing_generated_video_references_with_effects_record,
};
pub use normal::delete_generated_video_record_with_cleanup;
pub(crate) use references::clear_generated_video_references_by_ids;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoReferenceCleanupIds {
    pub updated_prompt_ids: Vec<i64>,
    pub updated_video_ids: Vec<i64>,
}
