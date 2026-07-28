use super::*;

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_no_existing_downstream_prompts_before_segment_planning(
    input: DownstreamSegmentPlanningGuardInput<'_>,
) -> Result<(), AppError> {
    let DownstreamSegmentPlanningGuardInput {
        service,
        project_id,
        chapter_id,
        spec,
        source_metadata,
        style,
        aspect_ratio,
        operation,
    } = input;
    let rows = a4n_db::video::chapter_video_prompt_records_for_workflow_types_by_prompt_and_shot(
        service.database.connection(),
        project_id,
        chapter_id,
        spec.workflow_mode.as_str(),
        &[spec.frame_prompt_type, "video_prompt"],
    )
    .await
    .map_err(AppError::Database)?;
    if let Some(row) = rows.into_iter().find(|row| {
        row_matches_segment_downstream_context(
            row,
            spec.workflow_mode,
            source_metadata,
            style,
            aspect_ratio,
        )
    }) {
        let later_label = if row.prompt_type == "video_prompt" {
            spec.video_prompt_label
        } else {
            spec.reused_frame_label
        };
        return Err(AppError::InvalidArgument(format!(
            "已存在后续{later_label} #{}，不能{operation}{}；请先删除后续{later_label}",
            row.shot_number, spec.segment_context
        )));
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) fn row_matches_segment_downstream_context(
    row: &chapter_video_prompt::Model,
    workflow_mode: PromptWorkflowMode,
    source_metadata: &Value,
    style: &str,
    aspect_ratio: &str,
) -> bool {
    if row.workflow_mode != workflow_mode.as_str()
        || row.style.as_deref() != Some(style)
        || row.aspect_ratio.as_deref() != Some(aspect_ratio)
    {
        return false;
    }
    source_hash_matches(
        prompt_row_source_hash(row),
        current_source_hash(source_metadata),
    )
}

pub(in crate::video::prompt_generation::frame_prompts) fn prompt_row_source_hash(
    row: &chapter_video_prompt::Model,
) -> Option<&str> {
    row.metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video::prompt_generation::frame_prompts) fn current_source_hash(
    source_metadata: &Value,
) -> Option<&str> {
    source_metadata
        .get("content_sha256")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video::prompt_generation::frame_prompts) fn source_hash_matches(
    row_hash: Option<&str>,
    current_hash: Option<&str>,
) -> bool {
    match (row_hash, current_hash) {
        (Some(row_hash), Some(current_hash)) => row_hash == current_hash,
        (None, None) => true,
        _ => false,
    }
}
