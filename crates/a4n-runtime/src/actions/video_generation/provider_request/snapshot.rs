mod capture;
mod contract;
mod record;

#[cfg(all(test, feature = "embedded-media-providers"))]
pub(in crate::actions::video_generation) use self::capture::chapter_video_provider_request_snapshot;
pub(in crate::actions::video_generation) use self::capture::chapter_video_provider_request_snapshot_for_state;
pub(in crate::actions::video_generation) use self::contract::provider_request_snapshot_value;
pub(in crate::actions::video_generation) use self::record::{
    model_request_preview_from_provider_snapshot, validate_video_provider_request_snapshot_owner,
};
