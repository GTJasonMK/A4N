mod chain;
mod frame_generation;

pub(super) use chain::{
    PromptDependencyFields, prompt_same_frame_reference_chain, prompt_same_generation_chain,
    prompt_stage, prompt_stage_label,
};
pub(super) use frame_generation::{
    ensure_frame_prompt_generation_source_matches, ensure_frame_prompt_workflow_matches_type,
    frame_image_generation_source_for_prompt,
};
#[cfg(test)]
mod tests;
