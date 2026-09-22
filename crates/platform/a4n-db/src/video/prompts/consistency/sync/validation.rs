use super::*;
use a4n_domain::video_prompt::ChapterVideoPromptSyncDraft;

pub(super) fn validate_prompt_sync_draft(
    project_id: i64,
    chapter_id: i64,
    draft: &ChapterVideoPromptSyncDraft,
) -> Result<(), VideoDbError> {
    let fields = &draft.fields;
    if fields.chapter_id != chapter_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频提示词草稿章节 {} 与同步章节 {} 不一致",
            fields.chapter_id, chapter_id
        )));
    }
    if fields.project_id != project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频提示词草稿项目 {} 与章节所属项目 {} 不一致",
            fields.project_id, project_id
        )));
    }
    validate_prompt_workflow_mode(&fields.workflow_mode, 0)?;
    if let Some(metadata_workflow_mode) = fields
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("workflow_mode"))
        .and_then(Value::as_str)
        && metadata_workflow_mode != fields.workflow_mode
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频提示词草稿 metadata.workflow_mode={} 与显式 workflow_mode={} 冲突",
            metadata_workflow_mode, fields.workflow_mode
        )));
    }
    let invalid_workflow = |expected: &str| {
        VideoDbError::InvalidArgument(format!(
            "视频提示词草稿类型 {} 必须属于 {} workflow，实际为 {}",
            fields.prompt_type, expected, fields.workflow_mode
        ))
    };
    match fields.prompt_type.as_str() {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        | chapter_video_prompt::PROMPT_TYPE_FRAME => {
            let valid =
                if fields.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN {
                    fields.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
                } else {
                    fields.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
                };
            if !valid {
                return Err(invalid_workflow(
                    if fields.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN {
                        chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
                    } else {
                        chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
                    },
                ));
            }
        }
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
            if fields.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN =>
        {
            return Err(invalid_workflow(
                chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
            ));
        }
        chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
            if fields.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO =>
        {
            return Err(invalid_workflow(
                chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            ));
        }
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD
            if fields.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_STORYBOARD =>
        {
            return Err(invalid_workflow(
                chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
            ));
        }
        chapter_video_prompt::PROMPT_TYPE_VIDEO => {
            let actual = fields
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("source_prompt_type"))
                .and_then(Value::as_str);
            if actual != Some(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN) {
                return Err(VideoDbError::InvalidArgument(format!(
                    "视频提示词草稿 workflow {} 的 video_prompt 必须声明 source_prompt_type=frame_segment_plan，实际为 {}",
                    fields.workflow_mode,
                    actual.unwrap_or("<missing>")
                )));
            }
        }
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
        | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
        | chapter_video_prompt::PROMPT_TYPE_STORYBOARD => {}
        other => {
            return Err(VideoDbError::InvalidArgument(format!(
                "未知视频提示词草稿类型：{other}"
            )));
        }
    }
    if fields.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        && prompt_execution_identity(fields.metadata_json.as_ref()).is_none()
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频提示词草稿类型 {} 缺少完整的稳定片段执行身份",
            fields.prompt_type
        )));
    }
    validate_prompt_output_language_metadata(
        &fields.prompt_type,
        fields.metadata_json.as_ref(),
        0,
    )?;
    Ok(())
}

pub(super) fn validate_prompt_output_language_metadata(
    prompt_type: &str,
    metadata: Option<&Value>,
    prompt_id: i64,
) -> Result<(), VideoDbError> {
    if !a4n_domain::prompt_language::prompt_type_uses_prompt_output_language(prompt_type) {
        return Ok(());
    }
    a4n_domain::prompt_language::PromptOutputLanguage::from_metadata(metadata).map_err(
        |error| {
            let subject = if prompt_id > 0 {
                format!("章节视频提示词 #{prompt_id}")
            } else {
                format!("视频提示词草稿类型 {prompt_type}")
            };
            VideoDbError::InvalidArgument(format!("{subject} {error}"))
        },
    )?;
    Ok(())
}

pub(super) fn validate_prompt_workflow_mode(
    workflow_mode: &str,
    prompt_id: i64,
) -> Result<(), VideoDbError> {
    if chapter_video_prompt::WORKFLOW_MODES.contains(&workflow_mode) {
        return Ok(());
    }
    let subject = if prompt_id > 0 {
        format!("章节视频提示词 #{prompt_id}")
    } else {
        "视频提示词草稿".to_string()
    };
    Err(VideoDbError::InvalidArgument(format!(
        "{subject} workflow_mode 无效: {workflow_mode}"
    )))
}
