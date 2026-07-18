mod facts;
mod identity;
mod query;
mod status;

pub(super) use query::video_project_progress_by_project;

const VIDEO_PROMPT_TYPE: &str = "video_prompt";
const VIDEO_FRAME_PROMPT_TYPE: &str = "frame_prompt";
const VIDEO_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
const CHAPTER_VIDEO_TYPE: &str = "chapter_video";

#[cfg(test)]
mod tests;
