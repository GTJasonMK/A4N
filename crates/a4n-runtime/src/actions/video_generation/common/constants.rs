pub(in crate::actions::video_generation) const VIDEO_GENERATION_PROGRESS_TOTAL: u32 = 7;
pub(in crate::actions::video_generation) const VIDEO_JOB_POLL_INTERVAL_SECS: u64 = 8;
pub(in crate::actions::video_generation) const VIDEO_JOB_MAX_WAIT_SECS: u64 = 30 * 60;
pub(in crate::actions::video_generation) const VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;
#[cfg(any(feature = "embedded-media-providers", test))]
pub(in crate::actions::video_generation) const OPENAI_COMPATIBLE_VIDEO_BACKEND: &str =
    "openai-compatible-video";
#[cfg(feature = "embedded-media-providers")]
pub(in crate::actions::video_generation) const OPENAI_CHAT_VIDEO_BACKEND: &str =
    "openai-chat-video";
pub(in crate::actions::video_generation) const NEWAPI_VIDEO_BACKEND: &str = "newapi-video";
#[cfg(any(feature = "embedded-media-providers", test))]
pub(in crate::actions::video_generation) const GEMINI_VEO_VIDEO_BACKEND: &str = "gemini-veo-video";
pub(in crate::actions::video_generation) const SEEDANCE2_VIDEO_BACKEND: &str = "seedance2-video";
pub(in crate::actions::video_generation) const CLIPROXY_OPENAI_VIDEO_BACKEND: &str =
    "cliproxy-openai-video";
pub(in crate::actions::video_generation) const MAX_CLIPROXY_VIDEO_PROMPT_CHARS: usize = 4096;
pub(in crate::actions::video_generation) const MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES: usize = 7;
pub(in crate::actions::video_generation) const MAX_SEEDANCE2_IMAGE_FILES: usize = 9;
pub(in crate::actions::video_generation) const MAX_SEEDANCE2_VIDEO_FILES: usize = 3;
pub(in crate::actions::video_generation) const MAX_SEEDANCE2_AUDIO_FILES: usize = 3;
pub(in crate::actions::video_generation) const VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER: &str = "[主体定义与素材绑定]\n【请求预览占位】当前视频 visual_prompt 未完整填写。真实生成前需要补全主体、输入帧或参考图用途、人物站位、场景锚点、构图和光影约束；此占位仅用于展示将发送给视频模型的请求结构。\n\n[片段内容以及前因后果]\n【请求预览占位】真实生成前需要补全片段承接状态、剧情动作、情绪转折和结尾交接状态。\n\n[镜头分镜]\n镜头 1：【请求预览占位】补全起始镜头，交代主体、景别、空间锚点和起始动作。\n镜头 2：【请求预览占位】补全收束镜头，交代动作桥、反应和结尾状态。\n\n[画面风格描述]\n【请求预览占位】真实生成前需要补全画面风格、光线、色彩、镜头质感和材质细节。\n\n[连续性与结尾状态]\n【请求预览占位】真实生成前需要补全人物身份、服装、道具、空间关系、光源方向和可承接尾态。\n\n[限制与避免项]\n【请求预览占位】不要配背景音乐，不要字幕、水印、角色变形、空间跳变或光影不连续。";
pub(in crate::actions::video_generation) const VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER: &str = "[运镜与动作]\n【请求预览占位】当前视频 motion_prompt 未完整填写。真实生成前需要补全机位、镜头运动、速度节奏、人物动作和转场方式；此占位仅用于展示将发送给视频模型的请求结构。\n\n[对白与表演]\n【请求预览占位】真实生成前需要补全对白意图、语气、口型、手势和听者反应；无对白时明确写无对白。";
pub(in crate::actions::video_generation) const REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER: &str = "【请求预览占位】当前参考媒体 prompt 未完整填写。真实生成前需要补全整体画面、动作、风格、镜头调度和每个参考媒体的用途；此占位仅用于展示将发送给视频模型的请求结构。";
pub(in crate::actions::video_generation) const PROVIDER_REQUEST_SNAPSHOT_METADATA_KEY: &str =
    "provider_request_snapshot";
pub(in crate::actions::video_generation) const CHAPTER_VIDEO_PROVIDER_PROMPT_NAME: &str =
    "chapter_video_provider_prompt";
pub(in crate::actions::video_generation) const CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME:
    &str = "chapter_video_first_frame_provider_prompt";
