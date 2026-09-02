mod contracts;
mod generation;
mod lifecycle;

pub use contracts::{
    BLUEPRINT_RELATIONSHIP_TYPES, ParsedBlueprint, ParsedBlueprintCharacter,
    ParsedBlueprintOptimization, ParsedBlueprintRelationship,
};
pub use generation::{
    BLUEPRINT_BASE_PROMPT, BLUEPRINT_GENERATE_PROGRESS_TOTAL, BLUEPRINT_GENERATION_KIND,
    BLUEPRINT_GENERATION_PROMPTS, BLUEPRINT_RAW_JSON_STREAM_ID, BLUEPRINT_REFINEMENT_FIELDS,
    BLUEPRINT_REFINEMENT_MODE, BLUEPRINT_REFINEMENT_PROMPT, BLUEPRINT_REFINEMENT_PROMPTS,
    BLUEPRINT_REFINEMENT_TASK_KIND, BLUEPRINT_REFINEMENT_USER_PROMPT,
    BLUEPRINT_REFINEMENT_WORKFLOW_KIND, BLUEPRINT_STAGE_SYSTEM_PROMPT, BLUEPRINT_STAGE_USER_PROMPT,
    blueprint_generation_input_fingerprint, blueprint_refinement_business_key,
    blueprint_refinement_call_key, blueprint_refinement_input_fingerprint,
    blueprint_stage_call_key, build_blueprint_generation_payload,
    normalized_blueprint_refinement_focus, validate_blueprint_chapter_count,
};
pub use lifecycle::{increment_blueprint_version, project_status_after_blueprint_generation};

#[cfg(test)]
mod tests;
