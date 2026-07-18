mod contracts;
mod messages;
mod source;

pub use contracts::{GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan};
pub use messages::{
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_script_adaptation_user_message, galgame_visual_planning_user_message,
};
pub use source::{
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_speaker_profiles_from_models,
};

#[cfg(test)]
mod tests;
