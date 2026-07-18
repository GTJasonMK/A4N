use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::video_execution::VideoShootingScriptIdentityV1;
use serde_json::{Value, json};

use crate::video::OrchestratorContext;
use crate::video::prompt_json::parse_stored_video_segment_plan_for_reuse;
use crate::video::prompt_store::{
    NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};

use super::super::{
    ReusableSegmentPlan, SHARED_SEGMENT_PLAN_TITLE, VideoShootingScriptProjection,
    current_source_hash, prompt_row_source_hash, segment_plan_rows, source_hash_matches,
    video_execution_chain_identity, video_projection_from_shooting_script,
};
use super::contracts::{
    ReusableSegmentPlanForPreviewQuery, ReusableSegmentPlanForSpecQuery,
    ReusableSegmentPlanFromRowInput, ReusableSegmentPlanQuery,
};

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_no_existing_segment_plan_before_new_planning(
    service: &OrchestratorContext,
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

pub(in crate::video::prompt_generation::frame_prompts) async fn reusable_segment_plan_for_resume(
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
        duration_secs,
        aspect_ratio,
        shooting_script_projection,
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
            duration_secs,
            aspect_ratio,
            shooting_script_projection,
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

pub(in crate::video::prompt_generation::frame_prompts) fn reusable_segment_plan_from_row(
    input: ReusableSegmentPlanFromRowInput<'_>,
) -> Result<ReusableSegmentPlan, AppError> {
    let ReusableSegmentPlanFromRowInput {
        row,
        source_metadata,
        source_spans,
        style,
        duration_secs,
        aspect_ratio,
        shooting_script_projection,
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
    if row.duration_secs != duration_secs {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} duration_secs={} 与当前请求 {} 不一致，不能用于续跑",
            row.shot_number, row.duration_secs, duration_secs
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
    if metadata.get("contract_version").and_then(Value::as_str) != Some(contract_version) {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} contract_version 不是当前合同 {contract_version}",
            row.shot_number
        )));
    }
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
    let expected_identity = video_execution_chain_identity(
        shooting_script_projection,
        &segment_plan,
        style,
        duration_secs,
        aspect_ratio,
    )?;
    let saved_identity = metadata.get("video_execution_identity").ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{context} #{} 缺少 video_execution_identity，不能用于续跑",
            row.shot_number
        ))
    })?;
    if saved_identity != &expected_identity {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} 的拍摄剧本、投影、片段或视频单元身份已变化，不能用于续跑",
            row.shot_number
        )));
    }
    let segment_json = json!({ "segments": &segment_plan });
    let canonical_segment_out = segment_json.to_string();
    metadata.insert("contract_version".to_string(), json!(contract_version));
    metadata.insert("video_execution_identity".to_string(), expected_identity);
    metadata.insert("segments".to_string(), json!(segment_plan));
    metadata.insert("source".to_string(), source_metadata.clone());
    let execution_identity = metadata
        .get("video_execution_identity")
        .cloned()
        .expect("validated execution identity must remain present");
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
        execution_identity,
    })
}

pub(in crate::video::prompt_generation::frame_prompts) async fn reusable_segment_plan_for_spec(
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
        duration_secs,
        aspect_ratio,
    } = query;
    let shooting_script_projection =
        current_video_shooting_script_projection(service, project_id, chapter_id, duration_secs)
            .await?;
    reusable_segment_plan_for_resume(ReusableSegmentPlanQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode: spec.segment_storage_workflow_mode(),
        prompt_type: spec.segment_storage_prompt_type(),
        source_metadata,
        source_spans,
        style,
        duration_secs,
        aspect_ratio,
        shooting_script_projection: &shooting_script_projection,
        contract_version: spec.segment_storage_contract_version(),
        context: spec.segment_storage_context(),
    })
    .await
}

pub(crate) async fn reusable_segment_plan_for_preview(
    query: ReusableSegmentPlanForPreviewQuery<'_>,
) -> Result<Option<ReusableSegmentPlan>, AppError> {
    let ReusableSegmentPlanForPreviewQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode,
        source_metadata,
        source_spans,
        style,
        duration_secs,
        aspect_ratio,
    } = query;
    let spec = super::super::frame_prompt_generation_spec_for_workflow(workflow_mode)?;
    let shooting_script_projection =
        current_video_shooting_script_projection(service, project_id, chapter_id, duration_secs)
            .await?;
    for row in segment_plan_rows(
        service,
        project_id,
        chapter_id,
        spec.segment_storage_workflow_mode(),
        spec.segment_storage_prompt_type(),
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
            duration_secs,
            aspect_ratio,
            shooting_script_projection: &shooting_script_projection,
            contract_version: spec.segment_storage_contract_version(),
            context: spec.segment_storage_context(),
            draft_workflow_mode: spec.segment_storage_workflow_mode(),
            draft_prompt_type: spec.segment_storage_prompt_type(),
            draft_title: spec.segment_storage_title(),
        }) {
            Ok(reusable) => return Ok(Some(reusable)),
            Err(err) => tracing::debug!(
                target: "a4n::video_prompt_preview",
                error = %err,
                "已保存视频片段规划不满足当前 v5 identity，预览不会复用"
            ),
        }
    }
    Ok(None)
}

async fn current_video_shooting_script_projection(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    duration_secs: i32,
) -> Result<VideoShootingScriptProjection, AppError> {
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
            },
        )
        .await?;
    let record = &resolved.shooting_script.record;
    video_projection_from_shooting_script(
        &resolved.shooting_script.script,
        VideoShootingScriptIdentityV1 {
            record_id: record.id,
            revision: record.revision,
            source_revision_key: record.source_revision_key.clone(),
            source_content_sha256: record.source_content_sha256.clone(),
            script_sha256: record.script_sha256.clone(),
        },
        duration_secs,
    )
}

pub(in crate::video::prompt_generation::frame_prompts) fn prompt_resume_context_matches(
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
