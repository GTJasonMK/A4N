mod paths;
mod playback;

pub use paths::MediaRoots;
pub use playback::{
    VIDEO_SCRUB_FRAMES_PER_SECOND, VideoScrubProxyError, execute, generate_video_scrub_frames,
    generate_video_scrub_proxy,
};
