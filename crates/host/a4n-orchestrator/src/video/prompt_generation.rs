mod frame_prompts;
mod llm_chat;

pub(crate) use self::frame_prompts::validate_video_inserted_segment_generation_options;
pub(in crate::video) use self::frame_prompts::video_projection_from_shooting_script;
pub(in crate::video) use self::frame_prompts::{
    generate_inserted_segment_draft, load_video_segment_plan_draft_checkpoints,
    load_video_segment_plan_draft_recipe_state, update_video_segment_plan_draft_recipe_checkpoint,
    video_execution_chain_identity, video_segment_execution_identity,
};
use self::llm_chat::{
    RenderedVideoPromptTrace, VideoPromptLlmCallTraceContext, VideoPromptRenderInput,
    VideoPromptStepProgress, build_video_prompt_request_with_call_trace,
    render_and_chat_trace_with_context, report_video_prompt_checkpoint,
    report_video_prompt_item_committed, report_video_prompt_progress,
};
use a4n_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
