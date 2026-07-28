use super::task::tauri_video_prompt_command;
use a4n_runtime::host_api::video_prompts as video_prompt_actions;

tauri_video_prompt_command!(
    generate_chapter_video_segment_plan,
    "正在准备片段规划生成",
    "片段规划生成已停止",
    video_prompt_actions::generate_chapter_video_segment_plan,
);

tauri_video_prompt_command!(
    generate_chapter_video_frame_prompts,
    "正在准备边界帧提示词生成",
    "边界帧提示词生成已停止",
    video_prompt_actions::generate_chapter_video_frame_prompts,
);

tauri_video_prompt_command!(
    generate_chapter_video_prompts,
    "正在准备视频提示词生成",
    "视频提示词生成已停止",
    video_prompt_actions::generate_chapter_video_prompts,
);
