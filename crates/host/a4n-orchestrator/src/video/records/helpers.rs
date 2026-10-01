//! assembly 与导入记录 helper。

use super::*;

pub(in crate::video) fn assembly_insert_position_metadata_from_rows(
    rows: &[GeneratedVideoRecord],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
    timeline_order: Option<&crate::video::timeline::VideoTimelineAssemblyOrder>,
    base_metadata: Option<Value>,
    insert_after_video_id: Option<i64>,
) -> Result<Value, AppError> {
    let anchors = assembly_clip_records_from_rows(
        rows,
        project_id,
        chapter_id,
        workflow_mode,
        excluded_video_id,
        adopted_video_ids,
        timeline_order,
    )?
    .into_iter()
    .map(|clip| AssemblyClipOrderAnchor {
        video_id: clip.video_id,
        sort_order: clip.sort_order,
        shot_number: clip.shot_number,
    })
    .collect::<Vec<_>>();
    let clip_ids = anchors.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
    let pushes = assembly_push_records(rows, workflow_mode).map_err(AppError::InvalidArgument)?;
    let active_source_ids = active_assembly_push_source_ids(
        pushes
            .iter()
            .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
    );
    ensure_insert_position_not_inside_pushed_prefix(
        &clip_ids,
        &active_source_ids,
        insert_after_video_id,
    )
    .map_err(AppError::InvalidArgument)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&anchors, insert_after_video_id)
            .map_err(AppError::InvalidArgument)?;
    assembly_insert_metadata_with_position(
        base_metadata,
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    )
    .map_err(AppError::InvalidArgument)
}

pub(in crate::video) fn required_workflow_mode<'a>(
    video: &'a GeneratedVideoRecord,
    label: &str,
) -> Result<&'a str, AppError> {
    video
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{label} #{} 缺少 workflow_mode", video.id))
        })
}

pub(in crate::video) fn assembly_clip_records_from_rows(
    rows: &[GeneratedVideoRecord],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
    timeline_order: Option<&crate::video::timeline::VideoTimelineAssemblyOrder>,
) -> Result<Vec<AssemblyClipRecord>, AppError> {
    let mut clips = rows
        .iter()
        .filter_map(|video| {
            let file_path = video.file_path.as_deref()?.trim();
            (Some(video.id) != excluded_video_id
                && video.project_id == project_id
                && video.chapter_id == Some(chapter_id)
                && if workflow_mode == WORKFLOW_MODE_MIXED {
                    adopted_video_ids.is_some_and(|ids| ids.contains(&video.id))
                        || (video.video_type
                            == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
                            && video.workflow_mode.as_deref() == Some(WORKFLOW_MODE_MIXED))
                } else {
                    video.workflow_mode.as_deref() == Some(workflow_mode)
                }
                && is_assembly_clip_video_type(&video.video_type)
                && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                && !file_path.is_empty())
            .then_some((video, file_path))
        })
        .map(|(video, file_path)| {
            let timeline_display_index =
                if video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT {
                    None
                } else {
                    timeline_order.and_then(|order| order.display_index_for_video(video))
                };
            if timeline_order.is_some()
                && video.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
                && timeline_display_index.is_none()
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{} 不属于当前章节视频时间线",
                    video.id
                )));
            }
            let (shot_number, sort_order) = match timeline_display_index {
                Some(index) => (index, f64::from(index) * 1000.0),
                None => (
                    assembly_clip_display_shot_number(
                        video.id,
                        &video.video_type,
                        video.metadata_json.as_ref(),
                    )
                    .map_err(AppError::InvalidArgument)?,
                    assembly_clip_sort_order(
                        video.id,
                        &video.video_type,
                        video.metadata_json.as_ref(),
                    )
                    .map_err(AppError::InvalidArgument)?,
                ),
            };
            Ok(AssemblyClipRecord {
                video_id: video.id,
                video_type: video.video_type.clone(),
                shot_number,
                sort_order,
                insert_after_video_id: assembly_clip_insert_after_video_id(
                    video.id,
                    &video.video_type,
                    video.metadata_json.as_ref(),
                )
                .map_err(AppError::InvalidArgument)?,
                file_path: file_path.to_string(),
                duration_secs: video.duration_secs,
                width: video.width,
                height: video.height,
                fps: video.fps,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    clips.sort_by(|left, right| {
        left.sort_order
            .total_cmp(&right.sort_order)
            .then_with(|| left.video_id.cmp(&right.video_id))
    });
    Ok(clips)
}

pub(in crate::video) fn next_assembly_push_status(
    action: AssemblyPushAction,
    status: &str,
) -> Result<&'static str, AppError> {
    match (action, status) {
        (AssemblyPushAction::Accept, ASSEMBLY_PUSH_STATUS_PENDING) => {
            Ok(ASSEMBLY_PUSH_STATUS_ACCEPTED)
        }
        (AssemblyPushAction::Reject, ASSEMBLY_PUSH_STATUS_PENDING) => {
            Ok(ASSEMBLY_PUSH_STATUS_REJECTED)
        }
        (AssemblyPushAction::Undo, ASSEMBLY_PUSH_STATUS_ACCEPTED) => {
            Ok(ASSEMBLY_PUSH_STATUS_PENDING)
        }
        (AssemblyPushAction::Accept, _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以同意".to_string(),
        )),
        (AssemblyPushAction::Reject, _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以拒绝".to_string(),
        )),
        (AssemblyPushAction::Undo, _) => Err(AppError::InvalidArgument(
            "只有已同意的最新推送可以撤销".to_string(),
        )),
    }
}

pub(in crate::video) fn imported_video_prompt_text(prompt: &ChapterVideoPromptRecord) -> String {
    let mut sections = vec![prompt.visual_prompt.trim().to_string()];
    if let Some(motion) = prompt
        .motion_prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sections.push(format!("Motion Prompt:\n{motion}"));
    }
    sections.join("\n\n")
}
