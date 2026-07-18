use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, finish_web_task, optional_string_field, register_web_task,
    sanitize_optional_task_id, string_action_result, video_prompt_actions, web_chunk_reporter,
    web_progress_reporter,
};
use super::input::video_prompt_input_from_args;
use super::task::video_prompt_web_task_command;

video_prompt_web_task_command!(
    generate_chapter_video_segment_plan,
    "正在准备片段规划生成",
    "片段规划生成完成",
    video_prompt_actions::generate_chapter_video_segment_plan,
);

video_prompt_web_task_command!(
    generate_chapter_video_frame_prompts,
    "正在准备边界帧提示词生成",
    "边界帧提示词生成完成",
    video_prompt_actions::generate_chapter_video_frame_prompts,
);

video_prompt_web_task_command!(
    generate_chapter_video_prompts,
    "正在准备视频提示词生成",
    "视频提示词生成完成",
    video_prompt_actions::generate_chapter_video_prompts,
);
