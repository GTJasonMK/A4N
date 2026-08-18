use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;

use super::*;

pub(in crate::asset_lineage_sync) fn video_generation_kinds(
    prompt_type: &str,
    workflow_mode: &str,
) -> Option<(&'static str, &'static str)> {
    match prompt_type {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN => {
            Some(("video_segment_plan", "video_segment_plan"))
        }
        chapter_video_prompt::PROMPT_TYPE_FRAME
        | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
        | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
        | chapter_video_prompt::PROMPT_TYPE_STORYBOARD => {
            Some(("video_frame_prompt", "image_prompt"))
        }
        chapter_video_prompt::PROMPT_TYPE_VIDEO
            if workflow_mode == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO =>
        {
            Some(("video_segment", "chat_video_prompt"))
        }
        chapter_video_prompt::PROMPT_TYPE_VIDEO => Some(("video_segment", "video_prompt")),
        _ => None,
    }
}

pub(in crate::asset_lineage_sync) fn video_prompt_artifact_text(
    row: &chapter_video_prompt::Model,
) -> String {
    let mut parts = vec![row.visual_prompt.clone()];
    if let Some(motion) = row
        .motion_prompt
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("【运动提示】\n{motion}"));
    }
    if let Some(negative) = row
        .negative_prompt
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("【负面提示】\n{negative}"));
    }
    parts.join("\n\n")
}

pub(super) fn should_sync_generated_image(
    image: &generated_image::Model,
    manga_prompt_by_image: &BTreeMap<i64, chapter_manga_prompt::Model>,
) -> bool {
    manga_prompt_by_image.contains_key(&image.id)
        || image.image_type == IMAGE_TYPE_VIDEO_TAIL_FRAME
}

pub(super) fn prompt_from_video_metadata<'a>(
    video: &generated_video::Model,
    prompts: &'a [chapter_video_prompt::Model],
) -> Option<&'a chapter_video_prompt::Model> {
    let prompt_id = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("prompt_id"))
        .and_then(Value::as_i64)?;
    prompts.iter().find(|prompt| prompt.id == prompt_id)
}

pub(in crate::asset_lineage_sync) fn canonical_spec_status(status: &str) -> Option<&'static str> {
    match status {
        "draft" => Some("draft"),
        "running" | "generating" => Some("running"),
        "completed" => Some("completed"),
        "failed" | "interrupted" => Some("failed"),
        "cancelled" => Some("cancelled"),
        "pending" => Some("ready"),
        _ => None,
    }
}

pub(in crate::asset_lineage_sync) fn canonical_media_status(status: &str) -> Option<&'static str> {
    match status {
        "pending" => Some("pending"),
        "running" | "generating" => Some("running"),
        "completed" => Some("completed"),
        "failed" | "interrupted" => Some("failed"),
        "cancelled" => Some("cancelled"),
        _ => None,
    }
}

pub(super) fn push_unsupported_prompt_type_warning(
    summary: &mut AssetLineageBackfillSummary,
    table: &str,
    row_id: i64,
    prompt_type: &str,
) {
    summary.warnings.push(format!(
        "{table} #{row_id} 的 prompt_type `{prompt_type}` 没有 canonical 映射，已跳过。"
    ));
}

pub(super) fn push_unsupported_status_warning(
    summary: &mut AssetLineageBackfillSummary,
    table: &str,
    row_id: i64,
    status: &str,
) {
    summary.warnings.push(format!(
        "{table} #{row_id} 的状态 `{status}` 无法映射到 canonical 状态，已跳过。"
    ));
}

pub(super) fn legacy_key(table: &str, id: i64, kind: &str) -> String {
    format!("legacy:{table}:{id}:{kind}")
}

pub(super) fn validate_positive_id(value: i64, label: &str) -> Result<(), AssetLineageSyncError> {
    if value > 0 {
        Ok(())
    } else {
        Err(AssetLineageSyncError::InvalidArgument(format!(
            "{label} 必须为正数"
        )))
    }
}
