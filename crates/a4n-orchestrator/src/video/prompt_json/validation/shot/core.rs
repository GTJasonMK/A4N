use a4n_error::AppError;

use a4n_domain::VideoAspectRatio;
use a4n_domain::prompt_language::PromptOutputLanguage;

use super::super::super::VideoPromptShot;
use super::super::prompt_text::{
    required_min_prompt_text, validate_motion_prompt_has_no_temporal_storyboard,
    validate_optional_prompt_text, validate_prompt_has_no_upload_status,
    validate_prompt_has_required_sections,
};
use super::super::{PromptTextKind, validate_reference_ids, validate_structured_value};
use super::dialogue::{DialogueTracePolicy, validate_dialogue_beats};
use super::references::validate_reference_binding;
use super::structure::{validate_self_contained_context, validate_transition_in};
use super::temporal::validate_temporal_beats;

pub(in crate::video::prompt_json) fn validate_shot(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot_with_dialogue_policy(
        shot,
        allowed_source_span_ids,
        prompt_output_language,
        DialogueTracePolicy::InternalNormalized,
    )
}

pub(in crate::video::prompt_json) fn validate_model_shot(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot_with_dialogue_policy(
        shot,
        allowed_source_span_ids,
        prompt_output_language,
        DialogueTracePolicy::ModelOutput,
    )
}

fn validate_shot_with_dialogue_policy(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
    dialogue_trace_policy: DialogueTracePolicy,
) -> Result<VideoPromptShot, AppError> {
    if shot.shot_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频提示词 shot_number 必须大于 0".to_string(),
        ));
    }
    if shot.title.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 title 不能为空".to_string(),
        ));
    }
    if shot.scene_summary.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 scene_summary 不能为空".to_string(),
        ));
    }
    validate_self_contained_context(&shot.self_contained_context, shot.shot_number)?;
    validate_structured_value(&shot.start_state, "视频提示词 start_state")?;
    validate_structured_value(&shot.end_state, "视频提示词 end_state")?;
    validate_transition_in(&shot.transition_in)?;
    let visual_prompt = required_min_prompt_text(
        shot.visual_prompt,
        "视频提示词 visual_prompt",
        prompt_output_language,
        PromptTextKind::VideoVisual,
    )?;
    let motion_prompt = required_min_prompt_text(
        shot.motion_prompt,
        "视频提示词 motion_prompt",
        prompt_output_language,
        PromptTextKind::VideoMotion,
    )?;
    validate_prompt_has_no_upload_status(&visual_prompt, "视频提示词 visual_prompt")?;
    validate_prompt_has_required_sections(
        &visual_prompt,
        "视频提示词 visual_prompt",
        "visual_prompt",
        prompt_output_language,
    )?;
    validate_prompt_has_no_upload_status(&motion_prompt, "视频提示词 motion_prompt")?;
    validate_prompt_has_required_sections(
        &motion_prompt,
        "视频提示词 motion_prompt",
        "motion_prompt",
        prompt_output_language,
    )?;
    validate_motion_prompt_has_no_temporal_storyboard(&motion_prompt)?;
    validate_optional_prompt_text(&shot.negative_prompt, "视频提示词 negative_prompt")?;
    validate_structured_value(&shot.blocking_plan, "视频提示词 blocking_plan")?;
    validate_structured_value(&shot.lighting_plan, "视频提示词 lighting_plan")?;
    validate_structured_value(
        &shot.emotion_action_beats,
        "视频提示词 emotion_action_beats",
    )?;
    let dialogue_beats = validate_dialogue_beats(
        &shot.dialogue_beats,
        allowed_source_span_ids,
        dialogue_trace_policy,
    )?;
    validate_structured_value(&shot.cinematography_plan, "视频提示词 cinematography_plan")?;
    validate_temporal_beats(&shot.temporal_beats, shot.duration_secs)?;
    validate_reference_binding(&shot.reference_binding, &shot.reference_image_ids)?;
    validate_structured_value(&shot.continuity_check, "视频提示词 continuity_check")?;
    if !(1..=60).contains(&shot.duration_secs) {
        return Err(AppError::InvalidArgument(
            "视频提示词 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    if shot.aspect_ratio.parse::<VideoAspectRatio>().is_err() {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 aspect_ratio 无效：{}，必须是 {}",
            shot.aspect_ratio,
            VideoAspectRatio::supported_values()
        )));
    }
    validate_reference_ids(&shot.reference_image_ids, "视频提示词 reference_image_ids")?;
    Ok(VideoPromptShot {
        title: shot.title.trim().to_string(),
        scene_summary: shot.scene_summary.trim().to_string(),
        visual_prompt,
        motion_prompt,
        negative_prompt: shot.negative_prompt.trim().to_string(),
        dialogue_beats,
        ..shot
    })
}
