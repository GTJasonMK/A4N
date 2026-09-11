use a4n_error::AppError;
use serde_json::{Map, Value, json};

use crate::video::prompt_json::VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS;

use super::super::contracts::VideoPromptCommonConstraintInput;
use super::super::references::video_prompt_reference_slots;
use super::selection::{
    video_prompt_dialogue_rule, video_prompt_duration_rule, video_prompt_shot_number_rule,
    video_prompt_temporal_rule,
};
use super::source::{insert_rule_value, rule_value};

pub(in crate::video::prompt_context::builders) fn final_video_prompt_output_contract(
    prompt_name: &str,
) -> Value {
    json!({
        "format": "json_object",
        "prompt_name": prompt_name,
        "required_root_field": "shots",
        "required_shot_count": 1,
        "required_shot_fields": [
            "title",
            "scene_summary",
            "visual_prompt",
            "motion_prompt",
            "negative_prompt",
            "dialogue_cues"
        ],
        "dialogue_cue_allowed_fields": VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS,
        "dialogue_cue_field_rule": "dialogue_cues 每一项只能包含 output_contract.dialogue_cue_allowed_fields 中列出的字段；不要输出额外字段。"
    })
}

pub(in crate::video::prompt_context::builders) fn video_prompt_common_constraints(
    input: VideoPromptCommonConstraintInput<'_>,
) -> Result<Map<String, Value>, AppError> {
    let VideoPromptCommonConstraintInput {
        rules,
        mode,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = input;
    let mut constraints = Map::new();
    constraints.insert("style".to_string(), json!(style));
    constraints.insert(
        "prompt_output_language".to_string(),
        prompt_output_language.to_prompt_json(),
    );
    constraints.insert(
        "prompt_output_language_instruction".to_string(),
        json!(prompt_output_language.instruction()),
    );
    constraints.insert("selected_duration_secs".to_string(), json!(duration_secs));
    constraints.insert(
        "duration_rule".to_string(),
        json!(video_prompt_duration_rule(rules, mode)?),
    );
    constraints.insert("aspect_ratio".to_string(), json!(aspect_ratio));
    constraints.insert(
        "style_preservation_rule".to_string(),
        rule_value(rules, "style_preservation_rule")?,
    );
    let reference_slots = video_prompt_reference_slots(reference_ids, mode)?;
    constraints.insert("reference_slots".to_string(), reference_slots);
    constraints.insert(
        "shot_number_rule".to_string(),
        json!(video_prompt_shot_number_rule(rules)?),
    );
    insert_rule_value(
        &mut constraints,
        "segment_directing_rule",
        rules,
        "segment_directing_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "material_coverage_rule",
        rules,
        "material_coverage_rule",
    )?;
    constraints.insert(
        "prompt_required_sections".to_string(),
        json!(prompt_output_language.visual_sections()),
    );
    insert_rule_value(
        &mut constraints,
        "prompt_text_section_rule",
        rules,
        "final_prompt_section_rule",
    )?;
    constraints.insert(
        "prompt_density_rule".to_string(),
        json!(prompt_output_language.density_rule()),
    );
    insert_rule_value(
        &mut constraints,
        "action_load_rule",
        rules,
        "action_load_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "time_budget_rule",
        rules,
        "time_budget_rule",
    )?;
    constraints.insert(
        "dialogue_rule".to_string(),
        json!(video_prompt_dialogue_rule(rules, mode)?),
    );
    insert_rule_value(
        &mut constraints,
        "sound_design_rule",
        rules,
        "sound_design_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "image_input_semantics_rule",
        rules,
        "image_prompt_semantics_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "input_frame_rule",
        rules,
        "input_frame_rule",
    )?;
    constraints.insert(
        "temporal_beat_rule".to_string(),
        json!(video_prompt_temporal_rule(rules, mode)?),
    );
    insert_rule_value(
        &mut constraints,
        "safe_visual_rewrite_rule",
        rules,
        "safe_visual_rewrite_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "reference_display_label_rule",
        rules,
        "reference_display_label_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "negative_prompt_rule",
        rules,
        "negative_prompt_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "natural_language_no_internal_fields_rule",
        rules,
        "natural_language_no_internal_fields_rule",
    )?;
    Ok(constraints)
}
