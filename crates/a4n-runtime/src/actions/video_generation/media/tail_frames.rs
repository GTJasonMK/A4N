mod capture;
mod deletion;
mod rules;

pub(crate) use self::capture::capture_video_tail_frame_asset;
pub(crate) use self::deletion::delete_video_tail_frame_assets;

#[cfg(test)]
pub(super) use self::rules::{tail_frame_image_belongs_to_video, video_shot_number};
