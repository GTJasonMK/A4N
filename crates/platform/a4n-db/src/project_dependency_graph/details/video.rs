mod frame_images;
mod generated_videos;
mod prompts;

pub(in crate::project_dependency_graph) use frame_images::video_frame_image_detail_records;
pub(in crate::project_dependency_graph) use generated_videos::generated_video_detail_records;
pub(in crate::project_dependency_graph) use prompts::video_prompt_detail_records;
