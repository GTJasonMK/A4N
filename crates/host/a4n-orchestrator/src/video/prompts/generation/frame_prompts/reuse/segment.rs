use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
use a4n_video_contract::{
    VideoShootingScriptIdentityV1, validate_video_segment_plan_identity,
    video_segment_plan_from_metadata,
};
use serde_json::{Value, json};

use crate::video::OrchestratorContext;
use crate::video::prompts::json::validate_stored_video_segment_plan;
use crate::video::prompts::store::{
    NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};

use super::super::{
    LoadedSharedSegmentPlan, ReusableSegmentPlan, SHARED_SEGMENT_PLAN_CONTRACT_VERSION,
    SHARED_SEGMENT_PLAN_TITLE, VideoShootingScriptProjection, segment_plan_rows,
    video_execution_chain_identity, video_projection_from_shooting_script,
};
use super::contracts::{
    ReusableSegmentPlanFromRowInput, ReusableSegmentPlanQuery, ReusableSharedSegmentPlanQuery,
};

pub(in crate::video::prompts::generation::frame_prompts) async fn load_shared_segment_plan_for_execution(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    segment_plan_prompt_id: i64,
) -> Result<LoadedSharedSegmentPlan, AppError> {
    let row = service
        .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
        .await?;
    let metadata = row
        .metadata_json
        .as_ref()
        .filter(|value| value.is_object())
        .ok_or_else(|| AppError::InvalidArgument("共享片段规划 metadata_json 无效".to_string()))?;
    if metadata.get("contract_version").and_then(Value::as_str)
        != Some(SHARED_SEGMENT_PLAN_CONTRACT_VERSION)
    {
        return Err(AppError::InvalidArgument(format!(
            "共享片段规划合同已过期，当前需要 {SHARED_SEGMENT_PLAN_CONTRACT_VERSION}"
        )));
    }
    let segment_plan =
        video_segment_plan_from_metadata(metadata).map_err(AppError::InvalidArgument)?;
    let execution_identity = metadata
        .get("video_execution_identity")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("共享片段规划缺少执行身份".to_string()))?;
    validate_video_segment_plan_identity(&execution_identity, &segment_plan)
        .map_err(AppError::InvalidArgument)?;
    Ok(LoadedSharedSegmentPlan {
        segment_plan,
        execution_identity,
    })
}

pub(in crate::video::prompts::generation::frame_prompts) async fn ensure_no_existing_segment_plan_before_new_planning(
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
    if let Some(row) = rows
        .iter()
        .find(|row| row.status == chapter_video_prompt::STATUS_RUNNING)
    {
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

pub(in crate::video::prompts::generation::frame_prompts) async fn reusable_segment_plan_for_resume(
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
    .filter(|row| row.status != chapter_video_prompt::STATUS_RUNNING)
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

pub(in crate::video::prompts::generation::frame_prompts) fn reusable_segment_plan_from_row(
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
    let metadata = metadata_json.as_object().ok_or_else(|| {
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
    let plan_complete = metadata
        .get("plan_complete")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "已保存{context} #{} 缺少有效的 plan_complete",
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
    let segment_plan = validate_stored_video_segment_plan(
        video_segment_plan_from_metadata(&metadata_json).map_err(AppError::InvalidArgument)?,
        source_spans,
        aspect_ratio,
    )?;
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
    validate_video_segment_plan_identity(saved_identity, &segment_plan)
        .map_err(AppError::InvalidArgument)?;
    if [
        "contract_epoch",
        "source_content_sha256",
        "style_sha256",
        "duration_secs",
        "aspect_ratio",
    ]
    .iter()
    .any(|field| saved_identity.get(*field) != expected_identity.get(*field))
    {
        return Err(AppError::InvalidArgument(format!(
            "已保存{context} #{} 的拍摄剧本、投影、片段或视频单元身份已变化，不能用于续跑",
            row.shot_number
        )));
    }
    let saved_source: VideoShootingScriptIdentityV1 =
        serde_json::from_value(saved_identity["shooting_script_identity"].clone())?;
    let plan_complete = plan_complete
        && segment_plan.iter().all(|segment| {
            saved_source.matches_shot(
                &shooting_script_projection
                    .shooting_script_identity
                    .source_content_sha256,
                &segment.source_shot_id,
                shooting_script_projection
                    .shooting_script_identity
                    .shot_fingerprints
                    .get(&segment.source_shot_id),
            )
        })
        && shooting_script_projection.shots().all(|shot| {
            segment_plan
                .iter()
                .any(|segment| segment.source_shot_id == shot.source_shot_id)
        });
    let segment_json = json!({ "segments": &segment_plan });
    let canonical_segment_out = segment_json.to_string();
    let metadata = metadata_json
        .as_object_mut()
        .expect("metadata object validated above");
    metadata.insert("contract_version".to_string(), json!(contract_version));
    metadata.insert("video_execution_identity".to_string(), expected_identity);
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
        plan_complete,
        draft,
    })
}

pub(in crate::video::prompts::generation::frame_prompts) async fn reusable_shared_segment_plan(
    query: ReusableSharedSegmentPlanQuery<'_>,
) -> Result<Option<ReusableSegmentPlan>, AppError> {
    let ReusableSharedSegmentPlanQuery {
        service,
        project_id,
        chapter_id,
        source_metadata,
        source_spans,
        style,
        duration_secs,
        aspect_ratio,
    } = query;
    let (shooting_script_projection, source_current) =
        current_video_shooting_script_projection(service, project_id, chapter_id).await?;
    let mut reused = reusable_segment_plan_for_resume(ReusableSegmentPlanQuery {
        service,
        project_id,
        chapter_id,
        workflow_mode: PromptWorkflowMode::SharedSegmentPlan,
        prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        source_metadata,
        source_spans,
        style,
        duration_secs,
        aspect_ratio,
        shooting_script_projection: &shooting_script_projection,
        contract_version: super::super::SHARED_SEGMENT_PLAN_CONTRACT_VERSION,
        context: super::super::SHARED_SEGMENT_PLAN_CONTEXT,
    })
    .await?;
    if let Some(plan) = &mut reused {
        plan.plan_complete &= source_current;
    }
    Ok(reused)
}

async fn current_video_shooting_script_projection(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
) -> Result<(VideoShootingScriptProjection, bool), AppError> {
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
                visual_plan_id: None,
            },
        )
        .await?;
    let record = &resolved.shooting_script.record;
    let confirmed = service
        .confirmed_visual_plan_for_shooting_script(&resolved.source, None)
        .await?;
    let director =
        crate::chapter_visual_plan::project_visual_plan_for_shooting_script(&confirmed.plan, &[])?;
    let source_current = resolved
        .shooting_script
        .script
        .plan_sync(&director)
        .is_current();
    let projection = video_projection_from_shooting_script(
        &resolved.shooting_script.script,
        VideoShootingScriptIdentityV1 {
            record_id: record.id,
            revision: record.revision,
            source_revision_key: record.source_revision_key.clone(),
            source_content_sha256: record.source_content_sha256.clone(),
            script_sha256: record.script_sha256.clone(),
            shot_fingerprints: resolved
                .shooting_script
                .script
                .shot_content_fingerprints(&director)?,
        },
    )?;
    Ok((projection, source_current))
}

pub(in crate::video::prompts::generation::frame_prompts) fn prompt_resume_context_matches(
    row: &ChapterVideoPromptRecord,
    source_metadata: &Value,
    style: &str,
    aspect_ratio: &str,
) -> bool {
    if row.style.as_deref() != Some(style) || row.aspect_ratio.as_deref() != Some(aspect_ratio) {
        return false;
    }
    prompt_row_source_hash(row)
        .zip(current_source_hash(source_metadata))
        .is_some_and(|(saved, current)| saved == current)
}

fn prompt_row_source_hash(row: &ChapterVideoPromptRecord) -> Option<&str> {
    row.metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn current_source_hash(source_metadata: &Value) -> Option<&str> {
    source_metadata
        .get("content_sha256")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::prompts::generation::frame_prompts::tests::{
        segment, shooting_script_projection,
    };

    #[test]
    fn execution_rejects_segment_plan_identity_without_stable_segment_identities() {
        let segment_plan = vec![segment(1)];
        let valid_identity = video_execution_chain_identity(
            &shooting_script_projection(),
            &segment_plan,
            "cinematic",
            5,
            "16:9",
        )
        .unwrap();
        validate_video_segment_plan_identity(&valid_identity, &segment_plan).unwrap();

        let mut forged = valid_identity.clone();
        forged["segment_identities"][0]["segment_sha256"] = json!("wrong-content");
        assert!(validate_video_segment_plan_identity(&forged, &segment_plan).is_err());
        let mut edited_segments = segment_plan.clone();
        edited_segments[0].scene_summary = "已经手工修改".into();
        assert!(validate_video_segment_plan_identity(&valid_identity, &edited_segments).is_err());

        let mut stale_identity = valid_identity;
        stale_identity
            .as_object_mut()
            .unwrap()
            .remove("segment_identities");
        assert!(
            validate_video_segment_plan_identity(&stale_identity, &segment_plan)
                .unwrap_err()
                .to_string()
                .contains("执行身份不完整")
        );
    }
}
