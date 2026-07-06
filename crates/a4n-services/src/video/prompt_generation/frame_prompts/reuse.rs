use super::*;

pub(super) fn ensure_video_prompts_can_regenerate(
    rows: &[chapter_video_prompt::Model],
    label: &str,
) -> Result<(), AppError> {
    if let Some(row) = rows.iter().find(|row| row.generated_video_id.is_some()) {
        return Err(AppError::InvalidArgument(format!(
            "已存在由{label} #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
            row.shot_number
        )));
    }
    Ok(())
}

pub(super) fn ensure_chat_video_prompts_can_generate_without_overwriting(
    all_existing_rows: &[chapter_video_prompt::Model],
    matching_rows: &[chapter_video_prompt::Model],
    reusable_prompts: &BTreeMap<i32, ChatVideoPromptShot>,
    segment_plan: &[VideoSegmentPlan],
    regenerate_existing: bool,
    generation_limit: usize,
) -> Result<(), AppError> {
    let mut allowed_reused_generated_row_ids = HashSet::new();
    let mut seen_matching_shots = HashSet::new();
    for row in matching_rows {
        if !seen_matching_shots.insert(row.shot_number) {
            continue;
        }
        let Some(reusable) = reusable_prompts.get(&row.shot_number) else {
            continue;
        };
        if !regenerate_existing
            && row.generated_video_id.is_some()
            && !reusable.chat_prompt.is_empty()
        {
            allowed_reused_generated_row_ids.insert(row.id);
        }
    }
    let existing_by_shot = all_existing_rows.iter().fold(
        HashMap::<i32, Vec<&chapter_video_prompt::Model>>::new(),
        |mut rows, row| {
            rows.entry(row.shot_number).or_default().push(row);
            rows
        },
    );
    let mut generated_this_run = 0usize;
    for segment in segment_plan {
        let shot_number = segment.segment_number;
        let reusable = reusable_prompts.get(&shot_number);
        if reusable.is_some() && (!regenerate_existing || generated_this_run >= generation_limit) {
            if let Some(rows) = existing_by_shot.get(&shot_number)
                && let Some(row) = rows.iter().find(|row| {
                    row.generated_video_id.is_some()
                        && !allowed_reused_generated_row_ids.contains(&row.id)
                })
            {
                return Err(AppError::InvalidArgument(format!(
                    "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                    row.shot_number
                )));
            }
            continue;
        } else if reusable.is_some()
            && regenerate_existing
            && generated_this_run >= generation_limit
        {
            continue;
        }

        if generated_this_run >= generation_limit {
            continue;
        }
        if let Some(rows) = existing_by_shot.get(&shot_number)
            && let Some(row) = rows.iter().find(|row| row.generated_video_id.is_some())
        {
            return Err(AppError::InvalidArgument(format!(
                "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                row.shot_number
            )));
        }
        generated_this_run += 1;
    }
    Ok(())
}

pub(super) async fn video_prompt_rows_for_workflow(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)
}

pub(super) async fn ensure_existing_frame_prompts_can_regenerate(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    for row in rows {
        if !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        ) {
            continue;
        }
        service
            .ensure_prompt_stage_mutable(&row, "重新生成")
            .await?;
    }
    Ok(())
}

pub(super) async fn ensure_existing_segment_plans_can_regenerate(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_prompt_ids_by_types(
        service,
        project_id,
        chapter_id,
        ignored_downstream_prompt_types,
        false,
    )
    .await?;
    let rows = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        false,
    )
    .await?;
    for row in rows {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}

pub(super) async fn ensure_existing_shot_analysis_can_regenerate(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    shot_analysis_prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_prompt_ids_by_types(
        service,
        project_id,
        chapter_id,
        ignored_downstream_prompt_types,
        false,
    )
    .await?;
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(shot_analysis_prompt_type))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    for row in rows {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}

pub(super) async fn ensure_segment_plan_generation_can_start(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_prompt_ids_by_types(
        service,
        project_id,
        chapter_id,
        ignored_downstream_prompt_types,
        true,
    )
    .await?;
    let existing = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        true,
    )
    .await?
    .into_iter()
    .next();
    if let Some(row) = existing {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}

pub(super) async fn ensure_shot_analysis_generation_can_start(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    shot_analysis_prompt_type: &str,
    ignored_downstream_prompt_types: &[&str],
) -> Result<(), AppError> {
    let ignored_prompt_ids = ignored_downstream_prompt_ids(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        ignored_downstream_prompt_types,
        false,
    )
    .await?;
    let existing = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(shot_analysis_prompt_type))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(0))
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .next();
    if let Some(row) = existing {
        service
            .ensure_prompt_stage_mutable_ignoring(&row, "重新生成", &ignored_prompt_ids)
            .await?;
    }
    Ok(())
}

pub(super) async fn ignored_downstream_prompt_ids(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_types: &[&str],
    ensure_mutable: bool,
) -> Result<BTreeSet<i64>, AppError> {
    if prompt_types.is_empty() {
        return Ok(BTreeSet::new());
    }
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    if ensure_mutable {
        for row in &rows {
            service.ensure_prompt_stage_mutable(row, "删除").await?;
        }
    }
    Ok(rows.into_iter().map(|row| row.id).collect())
}

pub(super) async fn ignored_prompt_ids_by_types(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
    ensure_mutable: bool,
) -> Result<BTreeSet<i64>, AppError> {
    if prompt_types.is_empty() {
        return Ok(BTreeSet::new());
    }
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    if ensure_mutable {
        for row in &rows {
            service.ensure_prompt_stage_mutable(row, "删除").await?;
        }
    }
    Ok(rows.into_iter().map(|row| row.id).collect())
}

pub(super) async fn segment_plan_rows(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    only_plan_row: bool,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let mut query = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type));
    if only_plan_row {
        query = query.filter(chapter_video_prompt::Column::ShotNumber.eq(0));
    }
    query
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)
}

pub(super) async fn ensure_no_existing_downstream_prompts_before_segment_planning(
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
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(spec.workflow_mode.as_str()))
        .filter(
            chapter_video_prompt::Column::PromptType
                .is_in([spec.frame_prompt_type, "video_prompt"]),
        )
        .order_by_asc(chapter_video_prompt::Column::PromptType)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(service.db.as_ref())
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

pub(super) fn row_matches_segment_downstream_context(
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

pub(super) fn prompt_row_source_hash(row: &chapter_video_prompt::Model) -> Option<&str> {
    row.metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn current_source_hash(source_metadata: &Value) -> Option<&str> {
    source_metadata
        .get("content_sha256")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn source_hash_matches(row_hash: Option<&str>, current_hash: Option<&str>) -> bool {
    match (row_hash, current_hash) {
        (Some(row_hash), Some(current_hash)) => row_hash == current_hash,
        (None, None) => true,
        _ => false,
    }
}

pub(super) struct ReusableSegmentPlanQuery<'a> {
    pub service: &'a VideoService,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub contract_version: &'a str,
    pub context: &'a str,
}

pub(super) struct ReusableSegmentPlanForSpecQuery<'a> {
    pub service: &'a VideoService,
    pub project_id: i64,
    pub chapter_id: i64,
    pub spec: &'a FramePromptGenerationSpec,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub aspect_ratio: &'a str,
}

pub(super) struct DownstreamSegmentPlanningGuardInput<'a> {
    pub service: &'a VideoService,
    pub project_id: i64,
    pub chapter_id: i64,
    pub spec: &'a FramePromptGenerationSpec,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub operation: &'a str,
}

pub(super) struct ReusableSegmentPlanFromRowInput<'a> {
    pub row: chapter_video_prompt::Model,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub contract_version: &'a str,
    pub context: &'a str,
    pub draft_workflow_mode: PromptWorkflowMode,
    pub draft_prompt_type: &'a str,
    pub draft_title: &'a str,
}

pub(super) struct ReusableFrameRowsQuery<'a> {
    pub service: &'a VideoService,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: Option<&'a Value>,
    pub source_prompt_type: Option<&'a str>,
    pub prompt_output_language: PromptOutputLanguage,
}

pub(in crate::video) struct FrameImageLookupContext<'a> {
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub source_prompt_type: Option<&'a str>,
    pub prompt_output_language: PromptOutputLanguage,
}

pub(in crate::video) struct FrameImageLookupRowsQuery<'a> {
    pub service: &'a VideoService,
    pub project_id: i64,
    pub chapter_id: i64,
    pub context: FrameImageLookupContext<'a>,
}

pub(super) async fn ensure_no_existing_segment_plan_before_new_planning(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    context: &str,
) -> Result<(), AppError> {
    let rows = segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        false,
    )
    .await?;
    if let Some(row) = rows.iter().find(|row| row.status == "running") {
        return Err(AppError::InvalidArgument(format!(
            "当前章节已有{context} #{} 正在运行，不能续跑或重新规划",
            row.shot_number
        )));
    }
    if let Some(row) = rows.into_iter().next() {
        return Err(AppError::InvalidArgument(format!(
            "已有{context} #{}，但结构无法复用；续跑不会自动删除或重建旧规划，请先确认后手动删除该规划及其后续记录再重新生成",
            row.shot_number
        )));
    }
    Ok(())
}

pub(super) async fn reusable_segment_plan_for_resume(
    query: ReusableSegmentPlanQuery<'_>,
) -> Result<Option<ReusableSegmentPlan>, AppError> {
    let ReusableSegmentPlanQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        source_metadata,
        source_spans,
        style,
        aspect_ratio,
        contract_version,
        context,
    } = query;
    let mut first_mismatch_error = None;
    for row in segment_plan_rows(
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        false,
    )
    .await?
    .into_iter()
    .filter(|row| row.status != "running")
    {
        match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
            row,
            source_metadata,
            source_spans,
            style,
            aspect_ratio,
            contract_version,
            context,
            draft_workflow_mode: workflow_mode,
            draft_prompt_type: prompt_type,
            draft_title: SHARED_SEGMENT_PLAN_TITLE,
        }) {
            Ok(reusable) => return Ok(Some(reusable)),
            Err(err) => {
                if first_mismatch_error.is_none() {
                    first_mismatch_error = Some(err);
                }
            }
        }
    }

    if let Some(err) = first_mismatch_error {
        return Err(AppError::InvalidArgument(format!(
            "已有{context}无法用于续跑：{err}；请删除该片段规划及其后续记录后重新生成"
        )));
    }
    Ok(None)
}

pub(super) fn reusable_segment_plan_from_row(
    input: ReusableSegmentPlanFromRowInput<'_>,
) -> Result<ReusableSegmentPlan, AppError> {
    let ReusableSegmentPlanFromRowInput {
        row,
        source_metadata,
        source_spans,
        style,
        aspect_ratio,
        contract_version,
        context,
        draft_workflow_mode,
        draft_prompt_type,
        draft_title,
    } = input;
    let saved_style = row.style.as_deref().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{context} #{} 缺少 style，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        ))
    })?;
    if saved_style != style {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} style 与当前请求不一致，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        )));
    }
    let saved_aspect_ratio = row.aspect_ratio.as_deref().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{context} #{} 缺少 aspect_ratio，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        ))
    })?;
    if saved_aspect_ratio != aspect_ratio {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} aspect_ratio 与当前请求不一致，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        )));
    }
    let mut metadata_json = row.metadata_json.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{context} #{} 缺少 metadata_json，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        ))
    })?;
    let metadata = metadata_json.as_object_mut().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{context} #{} 元数据格式无效",
            row.shot_number
        ))
    })?;
    let saved_source_hash = metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "已保存{context} #{} metadata_json 缺少 source.content_sha256，不能用于续跑；请删除旧规划并重新生成",
                row.shot_number
            ))
        })?;
    let request_source_hash = current_source_hash(source_metadata).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "当前视频源 metadata 缺少 content_sha256，不能复用{context}"
        ))
    })?;
    if saved_source_hash != request_source_hash {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} 来源内容与当前请求不一致，不能用于续跑；请删除旧规划并重新生成",
            row.shot_number
        )));
    }
    let segment_plan =
        parse_stored_video_segment_plan_for_reuse(&row.visual_prompt, source_spans, aspect_ratio)?;
    let segment_json = json!({ "segments": &segment_plan });
    let canonical_segment_out = segment_json.to_string();
    metadata.insert("contract_version".to_string(), json!(contract_version));
    metadata.insert("segments".to_string(), json!(segment_plan));
    metadata.insert("source".to_string(), source_metadata.clone());
    let draft = new_prompt_draft(NewChapterVideoPromptInput {
        project_id: row.project_id,
        chapter_id: row.chapter_id,
        workflow_mode: draft_workflow_mode,
        style: Some(style),
        prompt_type: draft_prompt_type,
        shot_number: row.shot_number,
        title: Some(draft_title),
        scene_summary: row.scene_summary.as_deref(),
        visual_prompt: &canonical_segment_out,
        motion_prompt: row.motion_prompt.as_deref(),
        negative_prompt: row.negative_prompt.as_deref(),
        duration_secs: row.duration_secs,
        aspect_ratio: Some(aspect_ratio),
        reference_image_ids_json: Some(json!([])),
        metadata_json: Some(metadata_json),
    });
    Ok(ReusableSegmentPlan {
        segment_json,
        segment_plan,
        draft,
    })
}

pub(super) async fn reusable_segment_plan_for_spec(
    query: ReusableSegmentPlanForSpecQuery<'_>,
) -> Result<Option<ReusableSegmentPlan>, AppError> {
    let ReusableSegmentPlanForSpecQuery {
        service,
        project_id,
        chapter_id,
        spec,
        source_metadata,
        source_spans,
        style,
        aspect_ratio,
    } = query;
    reusable_segment_plan_for_resume(ReusableSegmentPlanQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode: spec.segment_storage_workflow_mode(),
        prompt_type: spec.segment_storage_prompt_type(),
        source_metadata,
        source_spans,
        style,
        aspect_ratio,
        contract_version: spec.segment_storage_contract_version(),
        context: spec.segment_storage_context(),
    })
    .await
}

pub(super) fn prompt_resume_context_matches(
    row: &chapter_video_prompt::Model,
    source_metadata: &Value,
    style: &str,
    aspect_ratio: &str,
) -> bool {
    if row.style.as_deref() != Some(style) || row.aspect_ratio.as_deref() != Some(aspect_ratio) {
        return false;
    }
    source_hash_matches(
        prompt_row_source_hash(row),
        current_source_hash(source_metadata),
    )
}

pub(super) async fn reusable_frame_rows_for_resume(
    query: ReusableFrameRowsQuery<'_>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let ReusableFrameRowsQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        source_metadata,
        style,
        duration_secs,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint,
        source_prompt_type,
        prompt_output_language,
    } = query;
    service
        .reusable_prompt_rows(
            PromptWorkflowRowsContext {
                project_id,
                chapter_id,
                workflow_mode: workflow_mode.as_str(),
                prompt_type,
                source_metadata,
                style,
                duration_secs,
                aspect_ratio,
                reference_ids,
                generation_asset_fingerprint,
                source_prompt_type,
            },
            prompt_output_language,
        )
        .await
}

pub(in crate::video) async fn boundary_frame_rows_for_image_lookup(
    query: FrameImageLookupRowsQuery<'_>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let FrameImageLookupRowsQuery {
        service,
        project_id,
        chapter_id,
        context,
    } = query;
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(context.workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(context.prompt_type))
        .filter(chapter_video_prompt::Column::Status.ne("running"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let mut by_frame = BTreeMap::<i32, chapter_video_prompt::Model>::new();
    for row in rows {
        if !frame_row_matches_image_lookup_context(&row, &context) {
            continue;
        }
        by_frame.entry(row.shot_number).or_insert(row);
    }
    Ok(by_frame.into_values().collect())
}

pub(super) fn frame_row_matches_image_lookup_context(
    row: &chapter_video_prompt::Model,
    context: &FrameImageLookupContext<'_>,
) -> bool {
    prompt_resume_context_matches(
        row,
        context.source_metadata,
        context.style,
        context.aspect_ratio,
    ) && metadata_matches_prompt_output_language(
        row.metadata_json.as_ref(),
        context.prompt_output_language,
    ) && frame_prompt_row_matches_video_input_contract(
        row,
        context.workflow_mode,
        context.prompt_type,
        context.source_prompt_type,
    )
}

pub(super) async fn reusable_frame_rows_for_video_prompt_inputs(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    source_prompt_type: Option<&str>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::Status.ne("running"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let mut by_frame = BTreeMap::<i32, chapter_video_prompt::Model>::new();
    for row in rows {
        if !frame_prompt_row_matches_video_input_contract(
            &row,
            workflow_mode,
            prompt_type,
            source_prompt_type,
        ) {
            continue;
        }
        let row_matches_requested_language = metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        );
        match by_frame.get(&row.shot_number) {
            Some(existing)
                if row_matches_requested_language
                    && !metadata_matches_prompt_output_language(
                        existing.metadata_json.as_ref(),
                        prompt_output_language,
                    ) =>
            {
                by_frame.insert(row.shot_number, row);
            }
            Some(_) => {}
            None => {
                by_frame.insert(row.shot_number, row);
            }
        }
    }
    Ok(by_frame.into_values().collect())
}

pub(super) fn frame_prompt_row_matches_video_input_contract(
    row: &chapter_video_prompt::Model,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    source_prompt_type: Option<&str>,
) -> bool {
    if row.workflow_mode != workflow_mode.as_str() || row.prompt_type != prompt_type {
        return false;
    }
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let Some(expected_contract) = frame_prompt_contract_version(workflow_mode, prompt_type) else {
        return false;
    };
    if metadata.get("contract_version").and_then(Value::as_str) != Some(expected_contract) {
        return false;
    }
    source_prompt_type.is_none_or(|expected| {
        metadata.get("source_prompt_type").and_then(Value::as_str) == Some(expected)
    })
}

pub(super) fn frame_prompt_contract_version(
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
) -> Option<&'static str> {
    match (workflow_mode, prompt_type) {
        (PromptWorkflowMode::BoundaryFrames, "frame_prompt")
        | (PromptWorkflowMode::ChatVideo, "chat_video_frame_prompt") => {
            Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION)
        }
        (PromptWorkflowMode::FirstFrameChain, "first_frame_prompt") => {
            Some(FIRST_FRAME_CHAIN_CONTRACT_VERSION)
        }
        (PromptWorkflowMode::Storyboard, "storyboard_prompt") => {
            Some(STORYBOARD_PROMPT_CONTRACT_VERSION)
        }
        _ => None,
    }
}

pub(super) fn validate_frame_prompts_for_video_prompt_inputs(
    frame_prompts: Vec<VideoFramePrompt>,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let segment_by_number = segment_plan
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect::<HashMap<_, _>>();
    for frame in &frame_prompts {
        let Some(first_segment_number) = frame.segment_numbers.first().copied() else {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 缺少 segment_numbers，不能作为视频提示词输入",
                frame.frame_number
            )));
        };
        let Some(segment) = segment_by_number.get(&first_segment_number).copied() else {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 引用了不存在的片段 {}",
                frame.frame_number, first_segment_number
            )));
        };
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} aspect_ratio 必须等于片段 {} 的画幅 {}，实际为 {}；请重新生成{}",
                frame.frame_number,
                first_segment_number,
                segment.aspect_ratio,
                frame.aspect_ratio,
                label
            )));
        }
        if let Some(image_id) = frame
            .reference_image_ids
            .iter()
            .find(|image_id| !allowed_reference_ids.contains(image_id))
        {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 引用了当前上下文不可用的图片 ID：{}；请重新生成{}",
                frame.frame_number, image_id, label
            )));
        }
    }
    Ok(frame_prompts)
}

pub(super) async fn ensure_video_prompt_generation_can_start(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    shot_number: i32,
    label: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(shot_number))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    for row in rows {
        if !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        ) {
            continue;
        }
        if row.status == "running" {
            return Err(AppError::InvalidArgument(format!(
                "当前章节已有{label}正在运行，不能重新生成视频提示词"
            )));
        }
        service
            .ensure_prompt_stage_mutable(&row, "重新生成")
            .await?;
    }
    Ok(())
}

#[derive(Clone)]
pub(super) enum GeneratedVideoPromptShot {
    Api(Box<VideoPromptShot>),
    Chat(ChatVideoPromptShot),
}

impl GeneratedVideoPromptShot {
    pub(super) fn shot_number(&self) -> i32 {
        match self {
            Self::Api(prompt) => prompt.shot_number,
            Self::Chat(prompt) => prompt.shot_number,
        }
    }

    pub(super) fn to_json_value(&self) -> Result<Value, AppError> {
        match self {
            Self::Api(prompt) => serde_json::to_value(prompt.as_ref()),
            Self::Chat(prompt) => serde_json::to_value(prompt),
        }
        .map_err(|err| AppError::InvalidArgument(format!("视频提示词序列化失败：{err}")))
    }
}

pub(super) fn reusable_generated_video_prompts_from_rows(
    spec: &FramePromptGenerationSpec,
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, GeneratedVideoPromptShot>, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames => Ok(reusable_boundary_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| {
                (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
            })
            .collect()),
        PromptWorkflowMode::FirstFrameChain => {
            Ok(reusable_first_frame_video_prompts_from_rows(rows)?
                .into_iter()
                .map(|(shot_number, prompt)| {
                    (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
                })
                .collect())
        }
        PromptWorkflowMode::ChatVideo => Ok(reusable_chat_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| (shot_number, GeneratedVideoPromptShot::Chat(prompt)))
            .collect()),
        PromptWorkflowMode::Storyboard => Ok(reusable_boundary_video_prompts_from_rows(rows)?
            .into_iter()
            .map(|(shot_number, prompt)| {
                (shot_number, GeneratedVideoPromptShot::Api(Box::new(prompt)))
            })
            .collect()),
    }
}

pub(super) fn api_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    label: &str,
) -> Result<Vec<VideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|prompt| match prompt {
            GeneratedVideoPromptShot::Api(prompt) => Ok(prompt.as_ref().clone()),
            GeneratedVideoPromptShot::Chat(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 Chat 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(super) fn chat_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    label: &str,
) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|prompt| match prompt {
            GeneratedVideoPromptShot::Chat(prompt) => Ok(prompt.clone()),
            GeneratedVideoPromptShot::Api(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 API 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(super) fn chat_generated_video_prompt_map(
    prompts: &BTreeMap<i32, GeneratedVideoPromptShot>,
    label: &str,
) -> Result<BTreeMap<i32, ChatVideoPromptShot>, AppError> {
    prompts
        .iter()
        .map(|(shot_number, prompt)| match prompt {
            GeneratedVideoPromptShot::Chat(prompt) => Ok((*shot_number, prompt.clone())),
            GeneratedVideoPromptShot::Api(prompt) => Err(AppError::InvalidArgument(format!(
                "{label}链路收到 API 视频提示词 #{}，输出类型不匹配",
                prompt.shot_number
            ))),
        })
        .collect()
}

pub(super) fn parse_single_generated_video_prompt_shot(
    spec: &FramePromptGenerationSpec,
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    allowed_reference_ids: &HashSet<i64>,
    context: &str,
) -> Result<GeneratedVideoPromptShot, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => parse_single_video_prompt_shot(
            raw,
            segment_by_number,
            prompt_output_language,
            expected_shot_number,
            context,
        )
        .map(|prompt| GeneratedVideoPromptShot::Api(Box::new(prompt))),
        PromptWorkflowMode::ChatVideo => {
            let prompt = parse_single_chat_video_prompt_shot(
                raw,
                segment_by_number,
                prompt_output_language,
                expected_shot_number,
                context,
            )?;
            ensure_video_reference_ids_known(
                &prompt.reference_image_ids,
                allowed_reference_ids,
                &format!("{} #{}", spec.video_prompt_label, prompt.shot_number),
            )?;
            Ok(GeneratedVideoPromptShot::Chat(prompt))
        }
    }
}

pub(super) fn validate_generated_video_prompts(
    spec: &FramePromptGenerationSpec,
    prompts: &[GeneratedVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => validate_video_prompts(
            &api_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
        PromptWorkflowMode::ChatVideo => validate_chat_video_prompts(
            &chat_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
    }
}

pub(super) fn validate_partial_generated_video_prompts(
    spec: &FramePromptGenerationSpec,
    prompts: &[GeneratedVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames
        | PromptWorkflowMode::FirstFrameChain
        | PromptWorkflowMode::Storyboard => validate_partial_video_prompts(
            &api_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
        PromptWorkflowMode::ChatVideo => validate_partial_chat_video_prompts(
            &chat_generated_video_prompts(prompts, spec.video_prompt_label)?,
            segment_by_number,
            prompt_output_language,
        ),
    }
}

pub(in crate::video) fn video_prompt_segments_with_reference_ids(
    workflow_mode: PromptWorkflowMode,
    segments: &[VideoSegmentPlan],
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<VideoSegmentPlan> {
    segments
        .iter()
        .map(|segment| {
            let mut segment = segment.clone();
            segment.reference_image_ids = video_prompt_reference_ids_for_segment(
                workflow_mode,
                segment.segment_number,
                frame_by_number,
                frame_image_ids_by_number,
                generation_reference_ids,
            );
            segment
        })
        .collect()
}

pub(super) fn video_prompt_unit_segments_with_reference_ids(
    workflow_mode: PromptWorkflowMode,
    units: &[VideoUnitPlan],
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    units
        .iter()
        .map(|unit| {
            let mut segment = video_unit_as_prompt_segment(unit)?;
            segment.reference_image_ids = video_prompt_reference_ids_for_unit(
                workflow_mode,
                unit,
                frame_by_number,
                frame_image_ids_by_number,
                generation_reference_ids,
            );
            Ok(segment)
        })
        .collect()
}

fn video_unit_as_prompt_segment(unit: &VideoUnitPlan) -> Result<VideoSegmentPlan, AppError> {
    let mut segment = unit.representative_segment()?.clone();
    segment.segment_number = unit.unit_number;
    segment.title = unit.title.clone();
    retime_segment_performance_time_blocks(&mut segment, unit.duration_secs)?;
    segment.duration_secs = unit.duration_secs;
    if unit.unit_type == VideoUnitType::Transition {
        let from = unit
            .source_segments
            .first()
            .ok_or_else(|| AppError::InvalidArgument("过渡视频单元缺少前一片段".to_string()))?;
        let to = unit
            .source_segments
            .get(1)
            .ok_or_else(|| AppError::InvalidArgument("过渡视频单元缺少后一片段".to_string()))?;
        segment.scene_summary = format!(
            "过渡单元：从“{}”的尾态自然过渡到“{}”的首态，只补足时间、空间、动作或情绪交接，不新增关键剧情。",
            from.title, to.title
        );
        segment.blocking_summary = format!(
            "前一片段尾部调度：{}；后一片段开场调度：{}。本单元只设计两者之间的可见桥接。",
            from.blocking_summary, to.blocking_summary
        );
        segment.emotional_arc = json!({
            "start": from.emotional_arc.get("end").cloned().unwrap_or(Value::String("前一片段尾态".to_string())),
            "turn": "用动作、视线、空间移动或镜头转场完成片段间承上启下",
            "end": to.emotional_arc.get("start").cloned().unwrap_or(Value::String("后一片段首态".to_string())),
            "function": "片段过渡"
        });
        let mut seen_source_span_ids = HashSet::new();
        segment.source_span_ids = from
            .source_span_ids
            .iter()
            .chain(to.source_span_ids.iter())
            .filter(|id| seen_source_span_ids.insert((*id).clone()))
            .cloned()
            .collect();
    }
    Ok(segment)
}

fn retime_segment_performance_time_blocks(
    segment: &mut VideoSegmentPlan,
    duration_secs: i32,
) -> Result<(), AppError> {
    let Some(blocks) = segment
        .performance_plan
        .get_mut("time_blocks")
        .and_then(Value::as_array_mut)
    else {
        return Ok(());
    };
    if blocks.is_empty() {
        return Ok(());
    }

    let source_end_sec = blocks
        .last()
        .and_then(Value::as_object)
        .and_then(|object| object.get("end_sec"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频单元 #{} 无法从 performance_plan.time_blocks 推导原始时长",
                segment.segment_number
            ))
        })?;
    let scale = f64::from(duration_secs) / source_end_sec;
    let last_index = blocks.len().saturating_sub(1);
    let mut next_start_sec = 0.0_f64;

    for (index, block) in blocks.iter_mut().enumerate() {
        let field = format!(
            "视频单元 #{} time_blocks[{}]",
            segment.segment_number,
            index + 1
        );
        let object = block
            .as_object_mut()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是 JSON 对象")))?;
        let source_end = object
            .get("end_sec")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| AppError::InvalidArgument(format!("{field}.end_sec 必须是数字")))?;
        let end_sec = if index == last_index {
            f64::from(duration_secs)
        } else {
            (source_end * scale * 100.0).round() / 100.0
        };
        if end_sec <= next_start_sec {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 重定时后必须大于 start_sec"
            )));
        }
        object.insert("start_sec".to_string(), json!(next_start_sec));
        object.insert("end_sec".to_string(), json!(end_sec));
        next_start_sec = end_sec;
    }
    Ok(())
}

pub(super) fn video_prompt_reference_ids_for_unit(
    workflow_mode: PromptWorkflowMode,
    unit: &VideoUnitPlan,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<i64> {
    let mut refs = Vec::new();
    match workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            for frame_number in [unit.start_frame_number, unit.end_frame_number]
                .into_iter()
                .flatten()
            {
                if let Some(image_id) = frame_image_ids_by_number.get(&frame_number) {
                    extend_unique_reference_ids(&mut refs, &[*image_id]);
                }
                if let Some(frame) = frame_by_number.get(&frame_number) {
                    extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
                }
            }
        }
        PromptWorkflowMode::FirstFrameChain => {
            if unit.unit_number == 1
                && let Some(image_id) = frame_image_ids_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if unit.unit_number == 1
                && let Some(frame) = frame_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::Storyboard => {
            if let Some(storyboard_number) = unit.storyboard_number {
                if let Some(image_id) = frame_image_ids_by_number.get(&storyboard_number) {
                    extend_unique_reference_ids(&mut refs, &[*image_id]);
                }
                if let Some(frame) = frame_by_number.get(&storyboard_number) {
                    extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
                }
            }
        }
    }
    extend_unique_reference_ids(&mut refs, generation_reference_ids);
    refs
}

pub(super) fn video_prompt_reference_ids_for_segment(
    workflow_mode: PromptWorkflowMode,
    segment_number: i32,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<i64> {
    let mut refs = Vec::new();
    match workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            if let Some(image_id) = frame_image_ids_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(image_id) = frame_image_ids_by_number.get(&(segment_number + 1)) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(frame) = frame_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
            if let Some(frame) = frame_by_number.get(&(segment_number + 1)) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::FirstFrameChain => {
            if segment_number == 1
                && let Some(image_id) = frame_image_ids_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if segment_number == 1
                && let Some(frame) = frame_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::Storyboard => {
            if let Some(image_id) = frame_image_ids_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(frame) = frame_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
    }
    extend_unique_reference_ids(&mut refs, generation_reference_ids);
    refs
}

pub(super) fn extend_unique_reference_ids(target: &mut Vec<i64>, source: &[i64]) {
    for image_id in source {
        if *image_id > 0 && !target.contains(image_id) {
            target.push(*image_id);
        }
    }
}

pub(super) fn video_prompt_workflow_reference_ids(segments: &[VideoSegmentPlan]) -> Vec<i64> {
    let mut refs = Vec::new();
    for segment in segments {
        extend_unique_reference_ids(&mut refs, &segment.reference_image_ids);
    }
    refs
}

pub(super) fn image_generation_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

pub(super) fn image_generation_source(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn image_generation_shot_number(metadata: Option<&Value>) -> Option<i32> {
    let raw = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))?;
    raw.as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(super) fn image_generation_source_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn prompt_source_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) async fn source_spans_for_video_prompt(
    service: &VideoService,
    chapter_id: i64,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<Vec<SourceSpan>, AppError> {
    let chapter = service.get_chapter(chapter_id).await?;
    Ok(source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        source.content,
    ))
}

pub(in crate::video) async fn boundary_frame_image_ids_by_number(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    frame_rows: &[chapter_video_prompt::Model],
) -> Result<HashMap<i32, i64>, AppError> {
    let prompt_id_by_frame_number = frame_rows
        .iter()
        .map(|row| (row.id, row.shot_number))
        .collect::<HashMap<_, _>>();
    let source_hash_by_frame_number = frame_rows
        .iter()
        .filter_map(|row| {
            prompt_source_hash(row.metadata_json.as_ref()).map(|hash| (row.shot_number, hash))
        })
        .collect::<HashMap<_, _>>();
    if prompt_id_by_frame_number.is_empty() {
        return Ok(HashMap::new());
    }

    let allowed_project_ids = service.video_asset_project_ids(project_id).await?;
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(allowed_project_ids))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let mut image_id_by_frame_number = HashMap::new();
    for image in images {
        let metadata = image.metadata_json.as_ref();
        match image_generation_source(metadata) {
            Some("video_boundary_frame_prompt") => {
                let Some(prompt_id) = image_generation_prompt_id(metadata) else {
                    continue;
                };
                let Some(frame_number) = prompt_id_by_frame_number.get(&prompt_id).copied() else {
                    continue;
                };
                image_id_by_frame_number
                    .entry(frame_number)
                    .or_insert(image.id);
            }
            Some("video_tail_frame_capture") => {
                let Some(frame_number) = image_generation_shot_number(metadata)
                    .and_then(|shot_number| shot_number.checked_add(1))
                else {
                    continue;
                };
                let Some(expected_hash) = source_hash_by_frame_number.get(&frame_number) else {
                    continue;
                };
                if image_generation_source_hash(metadata) != Some(*expected_hash) {
                    continue;
                }
                image_id_by_frame_number
                    .entry(frame_number)
                    .or_insert(image.id);
            }
            _ => {}
        }
    }
    Ok(image_id_by_frame_number)
}

pub(in crate::video) async fn storyboard_frame_image_ids_by_number(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    frame_rows: &[chapter_video_prompt::Model],
) -> Result<HashMap<i32, i64>, AppError> {
    let prompt_id_by_frame_number = frame_rows
        .iter()
        .map(|row| (row.id, row.shot_number))
        .collect::<HashMap<_, _>>();
    if prompt_id_by_frame_number.is_empty() {
        return Ok(HashMap::new());
    }

    let allowed_project_ids = service.video_asset_project_ids(project_id).await?;
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(allowed_project_ids))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let mut image_id_by_frame_number = HashMap::new();
    for image in images {
        let metadata = image.metadata_json.as_ref();
        if image_generation_source(metadata) != Some("video_storyboard_prompt") {
            continue;
        }
        let Some(prompt_id) = image_generation_prompt_id(metadata) else {
            continue;
        };
        let Some(frame_number) = prompt_id_by_frame_number.get(&prompt_id).copied() else {
            continue;
        };
        image_id_by_frame_number
            .entry(frame_number)
            .or_insert(image.id);
    }
    Ok(image_id_by_frame_number)
}
