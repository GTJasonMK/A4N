mod owner;
mod payload;
mod requirements;

pub use owner::MediaEntryOwnerIndex;
pub(in crate::project::video_studio_package) use owner::validate_media_entry_owner;
pub(in crate::project::video_studio_package) use payload::validate_media_entry_bytes;
pub(super) use requirements::{require_media_payload, validate_optional_file_size};
