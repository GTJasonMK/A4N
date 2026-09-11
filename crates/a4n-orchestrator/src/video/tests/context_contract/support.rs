use super::*;
use a4n_domain::prompt_language::PromptOutputLanguage;
use std::future::Future;

const PROMPT_PROJECT_ID: i64 = 1;

trait FutureResultExt<T, E>: Future<Output = Result<T, E>> + Sized {
    fn expect(self, message: &str) -> T
    where
        E: std::fmt::Debug,
    {
        futures::executor::block_on(self).expect(message)
    }
}

impl<F, T, E> FutureResultExt<T, E> for F where F: Future<Output = Result<T, E>> + Sized {}

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
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
        super::super::prompt_context::FinalVideoPromptContext {
            prompt_renderer: $prompt_renderer,
            project_id: PROMPT_PROJECT_ID,
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
        super::super::prompt_context::FramePromptContext {
            prompt_renderer: $prompt_renderer,
            project_id: PROMPT_PROJECT_ID,
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
