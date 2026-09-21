mod contract;
mod record;

pub(in crate::actions::video_generation) use self::record::{
    model_request_preview_from_provider_snapshot, validate_video_provider_request_snapshot_owner,
};
