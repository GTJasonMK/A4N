use a4n_runtime::host_api::video_prompts::{
    self as video_prompt_actions, GeneratePromptOptionsInput,
};

pub(super) fn video_prompt_input(
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> video_prompt_actions::GenerateChapterVideoPromptInput {
    video_prompt_actions::GenerateChapterVideoPromptInput {
        user_id,
        project_id,
        chapter_id,
        options,
    }
}
