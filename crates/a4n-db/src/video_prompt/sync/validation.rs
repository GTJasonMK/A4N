use super::*;

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
        "frame_segment_plan" | "frame_prompt" => {
            let valid = if fields.prompt_type == "frame_segment_plan" {
                fields.workflow_mode == "shared_segment_plan"
            } else {
                fields.workflow_mode == "boundary_frames"
            };
            if !valid {
                return Err(invalid_workflow(
                    if fields.prompt_type == "frame_segment_plan" {
                        "shared_segment_plan"
                    } else {
                        "boundary_frames"
                    },
                ));
            }
        }
        "first_frame_prompt" if fields.workflow_mode != "first_frame_chain" => {
            return Err(invalid_workflow("first_frame_chain"));
        }
        "chat_video_frame_prompt" if fields.workflow_mode != "chat_video" => {
            return Err(invalid_workflow("chat_video"));
        }
        "storyboard_prompt" if fields.workflow_mode != "storyboard" => {
            return Err(invalid_workflow("storyboard"));
        }
        "video_prompt" => {
            let actual = fields
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("source_prompt_type"))
                .and_then(Value::as_str);
            if actual != Some("frame_segment_plan") {
                return Err(VideoDbError::InvalidArgument(format!(
                    "视频提示词草稿 workflow {} 的 video_prompt 必须声明 source_prompt_type=frame_segment_plan，实际为 {}",
                    fields.workflow_mode,
                    actual.unwrap_or("<missing>")
                )));
            }
        }
        "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt" => {}
        other => {
            return Err(VideoDbError::InvalidArgument(format!(
                "未知视频提示词草稿类型：{other}"
            )));
        }
    }
    if fields.prompt_type != "frame_segment_plan"
        && prompt_execution_identity(fields.metadata_json.as_ref()).is_none()
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频提示词草稿类型 {} 缺少完整的片段规划 ID、片段编号或 recipe 指纹",
            fields.prompt_type
        )));
    }
    Ok(())
}

pub(super) fn validate_prompt_workflow_mode(
    workflow_mode: &str,
    prompt_id: i64,
) -> Result<(), VideoDbError> {
    if matches!(
        workflow_mode,
        "shared_segment_plan"
            | "boundary_frames"
            | "first_frame_chain"
            | "chat_video"
            | "storyboard"
    ) {
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
