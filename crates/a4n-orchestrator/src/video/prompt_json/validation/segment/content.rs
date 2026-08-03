use a4n_core::{
    error::AppError,
    video_execution::{
        VideoEmotionActionGoalV1, VideoEmotionalArcV1, VideoIntentV1, VideoMaterialCoverageV1,
        VideoSpatialAnchorV1,
    },
};

use super::super::required_text;

pub(super) fn validate_segment_material_coverage(
    value: &VideoMaterialCoverageV1,
) -> Result<(), AppError> {
    for (index, entity) in value.required_entities.iter().enumerate() {
        required_text(
            entity.clone(),
            &format!(
                "视频分段规划 material_coverage.required_entities[{}]",
                index + 1
            ),
        )?;
    }
    for (index, entity) in value.missing_or_weak_references.iter().enumerate() {
        required_text(
            entity.clone(),
            &format!(
                "视频分段规划 material_coverage.missing_or_weak_references[{}]",
                index + 1
            ),
        )?;
    }
    required_text(
        value.risk_note.clone(),
        "视频分段规划 material_coverage.risk_note",
    )?;
    Ok(())
}

pub(super) fn validate_segment_spatial_anchors(
    items: &[VideoSpatialAnchorV1],
) -> Result<(), AppError> {
    if items.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划 spatial_anchors 不能为空".to_string(),
        ));
    }
    for (index, item) in items.iter().enumerate() {
        required_text(
            item.anchor.clone(),
            &format!("视频分段规划 spatial_anchors[{}].anchor", index + 1),
        )?;
        required_text(
            item.usage.clone(),
            &format!("视频分段规划 spatial_anchors[{}].usage", index + 1),
        )?;
    }
    Ok(())
}

pub(super) fn validate_segment_emotion_action_goals(
    items: &[VideoEmotionActionGoalV1],
) -> Result<(), AppError> {
    for (index, item) in items.iter().enumerate() {
        for (field, value) in [
            ("character", item.character.clone()),
            ("emotion", item.emotion.clone()),
            ("visible_action", item.visible_action.clone()),
        ] {
            required_text(
                value,
                &format!("视频分段规划 emotion_action_goals[{}].{}", index + 1, field),
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_segment_video_intent(value: &VideoIntentV1) -> Result<(), AppError> {
    for (field, text) in [
        ("hook", value.hook.clone()),
        ("audience_effect", value.audience_effect.clone()),
        ("rhythm", value.rhythm.clone()),
        ("visual_priority", value.visual_priority.clone()),
    ] {
        required_text(text, &format!("视频分段规划 video_intent.{field}"))?;
    }
    Ok(())
}

pub(super) fn validate_segment_emotional_arc(value: &VideoEmotionalArcV1) -> Result<(), AppError> {
    for (field, text) in [
        ("start", value.start.clone()),
        ("turn", value.turn.clone()),
        ("end", value.end.clone()),
        ("function", value.function.clone()),
    ] {
        required_text(text, &format!("视频分段规划 emotional_arc.{field}"))?;
    }
    Ok(())
}
