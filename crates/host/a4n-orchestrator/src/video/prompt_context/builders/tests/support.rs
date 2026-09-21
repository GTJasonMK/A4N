use std::future::Future;
use std::path::Path;

use a4n_prompt::tera_engine::TeraPromptEngine;
use serde_json::{Value, json};

use crate::video::prompt_json::VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS;
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::source_spans::source_spans_for_content;
use a4n_prompt::rendering::PromptRenderer;

use super::{
    FinalVideoPromptContext, FramePromptContext, VideoPlanningPromptContext,
    build_boundary_video_prompt_user, build_first_frame_video_prompt_user,
    build_segment_plan_for_shot_user, build_single_frame_prompt_user,
    render_strict_json_user_prompt,
};

const PROJECT_ID: i64 = 1;

trait FutureResultExt<T, E>: Future<Output = Result<T, E>> + Sized {
    fn expect(self, message: &str) -> T
    where
        E: std::fmt::Debug,
    {
        futures::executor::block_on(self).expect(message)
    }

    fn expect_err(self, message: &str) -> E
    where
        T: std::fmt::Debug,
    {
        futures::executor::block_on(self).expect_err(message)
    }
}

impl<F, T, E> FutureResultExt<T, E> for F where F: Future<Output = Result<T, E>> + Sized {}

fn planning_context<'a>(
    prompt_renderer: &'a dyn PromptRenderer,
    style: &'a str,
    duration_secs: i32,
) -> VideoPlanningPromptContext<'a> {
    VideoPlanningPromptContext {
        prompt_renderer,
        project_id: PROJECT_ID,
        chapter_summary: None,
        chapter_analysis: None,
        project_context: None,
        style,
        duration_secs,
        aspect_ratio: "16:9",
    }
}

macro_rules! final_video_context {
    (
        $prompt_renderer:expr,
        $segment_plan:expr,
        $frame_prompts:expr,
        $current_segment:expr,
        $previous_completed_shot:expr,
        $available_assets:expr,
        $style:expr,
        $prompt_output_language:expr,
        $duration_secs:expr,
        $aspect_ratio:expr,
        $reference_ids:expr $(,)?
    ) => {
        FinalVideoPromptContext {
            prompt_renderer: $prompt_renderer,
            project_id: PROJECT_ID,
            chapter_summary: None,
            chapter_analysis: None,
            segment_plan: $segment_plan,
            frame_prompts: $frame_prompts,
            current_segment: $current_segment,
            previous_completed_shot: $previous_completed_shot,
            available_assets: $available_assets,
            style: $style,
            prompt_output_language: $prompt_output_language,
            duration_secs: $duration_secs,
            aspect_ratio: $aspect_ratio,
            reference_ids: $reference_ids,
        }
    };
}

macro_rules! frame_prompt_context {
    (
        $prompt_renderer:expr,
        $segment_plan:expr,
        $available_assets:expr,
        $style:expr,
        $prompt_output_language:expr,
        $aspect_ratio:expr,
        $reference_ids:expr $(,)?
    ) => {
        FramePromptContext {
            prompt_renderer: $prompt_renderer,
            project_id: PROJECT_ID,
            chapter_summary: None,
            chapter_analysis: None,
            segment_plan: $segment_plan,
            available_assets: $available_assets,
            style: $style,
            prompt_output_language: $prompt_output_language,
            aspect_ratio: $aspect_ratio,
            reference_ids: $reference_ids,
        }
    };
}

fn prompt_renderer() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn video_user_payload(user: &str) -> Value {
    let payload = user
        .split_once("\n\n输入：\n")
        .map(|(_, payload)| payload)
        .expect("video user prompt should include payload delimiter");
    serde_json::from_str(payload.trim()).expect("video user payload should be JSON")
}

fn video_user_cache_prefix(user: &str) -> &str {
    user.split_once("\n\n输入：\n")
        .map(|(prefix, _)| prefix)
        .expect("video user prompt should include payload delimiter")
}

fn contains_json_key(value: &Value, target: &str) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| contains_json_key(item, target)),
        Value::Object(object) => {
            object.contains_key(target)
                || object
                    .values()
                    .any(|value| contains_json_key(value, target))
        }
        _ => false,
    }
}
