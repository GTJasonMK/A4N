fn timeline_item_context(
    item_id: Option<i64>,
    items: &[chapter_video_timeline_item::Model],
    prompts: &[ChapterVideoPromptTimelineRecord],
    segments: &[VideoSegmentPlanV2],
    recipes: &[VideoSegmentExecutionRecipeV1],
    videos: &[generated_video::Model],
    images: &[generated_image::Reference],
) -> Result<Value, AppError> {
    let Some(item_id) = item_id else {
        return Ok(Value::Null);
    };
    let item = items
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| AppError::InvalidState {
            current: "timeline_gap_item_missing".to_string(),
            operation: format!("读取插入片段相邻项 {item_id}"),
        })?;
    if item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS {
        return Err(AppError::InvalidState {
            current: format!("timeline_gap_item_inactive:{}", item.status),
            operation: format!("相邻时间线项 {item_id} 需要先重新定位"),
        });
    }
    let prompt = item
        .insert_plan_prompt_id
        .map(|id| {
            prompts
                .iter()
                .find(|prompt| prompt.id == id)
                .ok_or_else(|| AppError::InvalidState {
                    current: "inserted_timeline_prompt_missing".to_string(),
                    operation: format!("读取插入片段相邻项 {item_id}"),
                })
        })
        .transpose()?;
    let segment = if item.item_kind == a4n_db::video::TIMELINE_NATIVE_KIND {
        Some(
            segments
                .iter()
                .find(|segment| {
                    Some(segment.source_shot_id.as_str()) == item.native_source_shot_id.as_deref()
                        && Some(segment.segment_index_in_shot) == item.native_segment_index_in_shot
                })
                .ok_or_else(|| AppError::InvalidState {
                    current: "native_timeline_item_not_in_current_plan".to_string(),
                    operation: format!("读取插入片段相邻项 {item_id}"),
                })?,
        )
    } else {
        None
    };
    let selected_video_id = match item.item_kind.as_str() {
        a4n_db::video::TIMELINE_INSERTED_KIND => {
            prompt.and_then(|prompt| prompt.generated_video_id)
        }
        a4n_db::video::TIMELINE_NATIVE_KIND => segment.and_then(|segment| {
            recipes
                .iter()
                .find(|recipe| recipe.segment_number == segment.segment_number)
                .and_then(|recipe| recipe.adopted_video_id)
        }),
        other => {
            return Err(AppError::InvalidState {
                current: format!("unknown_timeline_item_kind:{other}"),
                operation: format!("读取插入片段相邻项 {item_id}"),
            });
        }
    };
    let video_id = current_timeline_item_video_id(item, selected_video_id, videos)?;
    let tail_frame_image_id = video_id.and_then(|video_id| {
        images
            .iter()
            .find(|image| crate::video::tail_frame_reference_belongs_to_video(image, video_id))
            .map(|image| image.id)
    });
    Ok(json!({
        "id": item.id,
        "kind": item.item_kind,
        "title": prompt.and_then(|prompt| prompt.title.clone())
            .or_else(|| segment.map(|segment| segment.title.clone())),
        "scene_summary": prompt.and_then(|prompt| prompt.scene_summary.clone())
            .or_else(|| segment.map(|segment| segment.scene_summary.clone())),
        "visual_prompt": prompt.map(|prompt| prompt.visual_prompt.clone()),
        "motion_prompt": prompt.and_then(|prompt| prompt.motion_prompt.clone()),
        "segment": segment,
        "video": video_id.map(|video_id| json!({
            "video_id": video_id,
            "tail_frame_image_id": tail_frame_image_id,
        })),
    }))
}

pub(crate) fn current_timeline_item_video_id(
    item: &chapter_video_timeline_item::Model,
    current_video_id: Option<i64>,
    videos: &[generated_video::Model],
) -> Result<Option<i64>, AppError> {
    let Some(video_id) = current_video_id else {
        return Ok(None);
    };
    let video = videos
        .iter()
        .find(|video| video.id == video_id)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("timeline_selected_video_missing:{video_id}"),
            operation: format!("读取时间线项 {} 的当前视频", item.id),
        })?;
    if video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        || !video_matches_timeline_item(video, item)
    {
        return Err(AppError::InvalidState {
            current: format!("timeline_selected_video_invalid:{video_id}"),
            operation: format!("读取时间线项 {} 的当前视频", item.id),
        });
    }
    Ok(Some(video_id))
}

pub(crate) fn shared_plan_segments(
    plan: &a4n_db_models::models::chapter::chapter_video_prompt::Model,
) -> Result<Vec<VideoSegmentPlanV2>, AppError> {
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(
        plan.metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("segments"))
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument("当前共享片段规划缺少 segments".to_string())
            })?,
    )
    .map_err(|error| AppError::InvalidArgument(format!("当前共享片段规划无效：{error}")))?;
    if segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "当前共享片段规划不能是空规划".to_string(),
        ));
    }
    Ok(segments)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentWorkflowFingerprint {
    schema_version: i32,
    workflow_kind: String,
    project_id: i64,
    chapter_id: i64,
    source_content_sha256: String,
    context_sha256: String,
    timeline_id: i64,
    timeline_revision: i64,
    gap: VideoTimelineGap,
    payload: InsertedSegmentPayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentPayload {
    user_prompt: String,
    style: String,
    prompt_output_language: String,
    constraints: InsertedSegmentConstraints,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentConstraints {
    selected_duration_secs: i32,
    duration_secs_options: Vec<i32>,
    aspect_ratio: String,
    guidance_mode: VideoSegmentGuidanceModeV1,
    continuity_source_video_id: Option<i64>,
    reference_image_ids: Vec<i64>,
}

fn validate_inserted_workflow_scope(
    run: &a4n_db_models::models::generation::workflow_run::Model,
    input: &CommitInsertedSegmentDraftInput,
) -> Result<(), AppError> {
    if run.workflow_kind != INSERTED_SEGMENT_WORKFLOW_KIND {
        return Err(AppError::InvalidArgument(format!(
            "workflow #{} 不是插入片段草稿",
            run.id
        )));
    }
    if run.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: run.status.clone(),
            operation: "提交尚未完成的插入片段草稿".to_string(),
        });
    }
    if run.scope_table.as_deref() != Some(VIDEO_TIMELINE_SCOPE_TABLE)
        || run.scope_id != Some(input.timeline_id)
    {
        return Err(AppError::InvalidArgument(
            "插入片段 workflow 不属于目标时间线".to_string(),
        ));
    }
    Ok(())
}

fn inserted_workflow_fingerprint(
    run: &a4n_db_models::models::generation::workflow_run::Model,
    value: &Value,
) -> Result<InsertedSegmentWorkflowFingerprint, AppError> {
    let fingerprint: InsertedSegmentWorkflowFingerprint = serde_json::from_value(value.clone())
        .map_err(|error| {
            AppError::InvalidArgument(format!("插入片段 workflow 指纹无效: {error}"))
        })?;
    if fingerprint.schema_version != 1
        || fingerprint.workflow_kind != INSERTED_SEGMENT_WORKFLOW_KIND
        || Some(fingerprint.project_id) != run.project_id
        || Some(fingerprint.chapter_id) != run.chapter_id
    {
        return Err(AppError::InvalidState {
            current: "inserted_segment_workflow_fingerprint_scope_mismatch".to_string(),
            operation: "提交插入片段草稿".to_string(),
        });
    }
    if fingerprint.source_content_sha256.trim().is_empty()
        || fingerprint.context_sha256.trim().is_empty()
        || fingerprint.payload.user_prompt.trim().is_empty()
    {
        return Err(AppError::InvalidState {
            current: "inserted_segment_workflow_fingerprint_incomplete".to_string(),
            operation: "提交插入片段草稿".to_string(),
        });
    }
    Ok(fingerprint)
}

fn inserted_segment_recipe(
    fingerprint: &InsertedSegmentWorkflowFingerprint,
    segment_number: i32,
    prompt_language: PromptOutputLanguage,
) -> Result<VideoSegmentExecutionRecipeV1, AppError> {
    let constraints = &fingerprint.payload.constraints;
    let guidance_mode = constraints.guidance_mode;
    let workflow_mode = inserted_segment_workflow_mode(guidance_mode)?;
    let (reference_image_ids, guidance_reference_image_ids, continuity_source_video_id) =
        match guidance_mode {
            VideoSegmentGuidanceModeV1::Keyframes => {
                (Vec::new(), constraints.reference_image_ids.clone(), None)
            }
            VideoSegmentGuidanceModeV1::Continuity => (
                constraints.reference_image_ids.clone(),
                Vec::new(),
                constraints.continuity_source_video_id,
            ),
            VideoSegmentGuidanceModeV1::None | VideoSegmentGuidanceModeV1::SystemFrames => {
                (Vec::new(), Vec::new(), None)
            }
        };
    let recipe = VideoSegmentExecutionRecipeV1 {
        schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
        segment_number,
        workflow_mode,
        style: fingerprint.payload.style.trim().to_string(),
        prompt_output_language: prompt_language.as_str().to_string(),
        duration_secs: constraints.selected_duration_secs,
        duration_secs_options: constraints.duration_secs_options.clone(),
        aspect_ratio: constraints.aspect_ratio.clone(),
        reference_image_ids,
        video_library_image_ids: Vec::new(),
        guidance_mode,
        guidance_reference_image_ids,
        continuity_source_video_id,
        reference_media: None,
        adopted_video_id: None,
    };
    recipe.validate().map_err(AppError::InvalidArgument)?;
    recipe
        .validate_generation_ready()
        .map_err(AppError::InvalidArgument)?;
    Ok(recipe)
}

pub(crate) fn inserted_segment_workflow_mode(
    guidance_mode: VideoSegmentGuidanceModeV1,
) -> Result<VideoSegmentWorkflowModeV1, AppError> {
    match guidance_mode {
        VideoSegmentGuidanceModeV1::Continuity => Ok(VideoSegmentWorkflowModeV1::FirstFrameChain),
        VideoSegmentGuidanceModeV1::Keyframes | VideoSegmentGuidanceModeV1::None => {
            Ok(VideoSegmentWorkflowModeV1::BoundaryFrames)
        }
        VideoSegmentGuidanceModeV1::SystemFrames => Err(AppError::InvalidArgument(
            "插入片段不支持 system_frames 引导".to_string(),
        )),
    }
}

async fn validate_inserted_continuity_source(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    recipe: &VideoSegmentExecutionRecipeV1,
) -> Result<(), AppError> {
    let Some(video_id) = recipe.continuity_source_video_id else {
        return Ok(());
    };
    let videos = service
        .generated_videos_for_chapter(project_id, chapter_id)
        .await?;
    let video = videos
        .iter()
        .find(|video| {
            video.id == video_id
                && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        })
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "插入片段续接来源视频 #{video_id} 已失效，请刷新时间线后重试"
            ))
        })?;
    if video.project_id != project_id || video.chapter_id != Some(chapter_id) {
        return Err(AppError::InvalidArgument(
            "插入片段续接来源视频不属于当前章节".to_string(),
        ));
    }
    let images = service
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await?;
    if !images
        .iter()
        .any(|image| crate::video::tail_frame_reference_belongs_to_video(image, video_id))
    {
        return Err(AppError::InvalidArgument(format!(
            "插入片段续接来源视频 #{video_id} 已没有可用尾帧，请刷新后重试"
        )));
    }
    Ok(())
}

pub(crate) fn timeline_revision_conflict(current: i64, expected: i64) -> AppError {
    AppError::InvalidState {
        current: format!("timeline_revision_conflict:current={current}"),
        operation: format!("expected={expected}"),
    }
}

fn native_timeline_needs_reconcile(
    snapshot: &a4n_db::video::VideoTimelineSnapshot,
    keys: &[NativeVideoTimelineKey],
) -> Result<bool, AppError> {
    let order =
        a4n_db::video::active_timeline_order(snapshot).map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "检查章节视频时间线同步状态".to_string(),
        })?;
    let by_id = snapshot
        .items
        .iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();
    let mut native = Vec::with_capacity(keys.len());
    for item_id in order {
        let Some(item) = by_id.get(&item_id) else {
            return Ok(true);
        };
        if item.item_kind != a4n_db::video::TIMELINE_NATIVE_KIND {
            continue;
        }
        let (Some(source_shot_id), Some(segment_index)) = (
            item.native_source_shot_id.as_deref(),
            item.native_segment_index_in_shot,
        ) else {
            return Ok(true);
        };
        if item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS {
            return Ok(true);
        }
        native.push((source_shot_id, segment_index));
    }
    if native.len() != keys.len() {
        return Ok(true);
    }
    Ok(native.iter().zip(keys).any(|((shot, index), key)| {
        *shot != key.source_shot_id || *index != key.segment_index_in_shot
    }))
}
