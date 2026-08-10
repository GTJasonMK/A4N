use std::collections::HashMap;

use a4n_api_contract::video::VideoInsertedSegmentGenerationOptions;
use a4n_core::{
    error::AppError,
    prompt_language::PromptOutputLanguage,
    source_spans::source_text_hash,
    video_execution::{VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1, timeline_gap_index},
};
use a4n_llm::context::LlmContextLedger;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::GenerationResultRef;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    workflow::{
        ClaimExclusiveWorkflowRunInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING,
        WorkflowRunFinalizeOutcome,
    },
};

use super::super::super::prompt_json::InsertedSegmentDraft;
use super::{
    RenderOrRestoreJsonWithValidationInput, VideoPromptChunk, VideoPromptProgress,
    VideoPromptStepProgress, render_or_restore_json_with_validation_with_context,
    report_video_prompt_progress,
};

const INSERTED_SEGMENT_PROMPT_NAME: &str = "chapter_video_inserted_segment";

pub(crate) fn validate_video_inserted_segment_generation_options(
    options: &VideoInsertedSegmentGenerationOptions,
) -> Result<(), AppError> {
    if options.timeline_id <= 0 || options.expected_timeline_revision <= 0 {
        return Err(AppError::InvalidArgument(
            "插入片段必须提供有效的时间线 ID 和 revision".to_string(),
        ));
    }
    if options.previous_item_id.is_some_and(|id| id <= 0)
        || options.next_item_id.is_some_and(|id| id <= 0)
    {
        return Err(AppError::InvalidArgument(
            "插入片段前后项 ID 必须为正整数".to_string(),
        ));
    }
    if options.previous_item_id.is_some() && options.previous_item_id == options.next_item_id {
        return Err(AppError::InvalidArgument(
            "插入片段前后项不能是同一项".to_string(),
        ));
    }
    if options.user_prompt.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "插入片段用户提示词不能为空".to_string(),
        ));
    }
    if options.style.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "插入片段 style 不能为空".to_string(),
        ));
    }
    if options.duration_secs <= 0 || options.duration_secs > 60 {
        return Err(AppError::InvalidArgument(
            "插入片段 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    if options.duration_secs_options.is_empty()
        || options
            .duration_secs_options
            .iter()
            .any(|duration| *duration <= 0 || *duration > 60)
    {
        return Err(AppError::InvalidArgument(
            "插入片段 duration_secs_options 不能为空且必须全部在 1 到 60 秒之间".to_string(),
        ));
    }
    let duration_options = normalized_duration_options(options);
    if !duration_options.contains(&options.duration_secs) {
        return Err(AppError::InvalidArgument(
            "插入片段 duration_secs 必须包含在 duration_secs_options 中".to_string(),
        ));
    }
    if options.aspect_ratio.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "插入片段 aspect_ratio 不能为空".to_string(),
        ));
    }
    crate::video::ensure_reference_image_ids_valid(&options.reference_image_ids)
        .map_err(AppError::InvalidArgument)?;
    Ok(())
}

pub(crate) async fn generate_inserted_segment_draft(
    service: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    active_task_id: &str,
    root_project_id: i64,
    retry_of_task_id: Option<&str>,
    options: VideoInsertedSegmentGenerationOptions,
    prompt_renderer: &dyn PromptRenderer,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: &CancellationToken,
) -> Result<GenerationResultRef, AppError> {
    validate_video_inserted_segment_generation_options(&options)?;
    service
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
        .await?;
    let source_bundle = service
        .project_chapter_video_source(project_id, chapter_id, "生成插入片段草稿", false)
        .await?;
    let source_content_sha256 = source_text_hash(&source_bundle.source.content);
    let snapshot = service
        .chapter_video_timeline_snapshot(options.timeline_id)
        .await?;
    if snapshot.timeline.project_id != project_id || snapshot.timeline.chapter_id != chapter_id {
        return Err(AppError::InvalidArgument(
            "插入片段时间线不属于当前项目章节".to_string(),
        ));
    }
    if snapshot.timeline.workflow_mode != crate::video::timeline::SHARED_VIDEO_TIMELINE_MODE {
        return Err(AppError::InvalidArgument(
            "插入片段只支持当前共享片段规划时间线".to_string(),
        ));
    }
    if snapshot.timeline.revision != options.expected_timeline_revision {
        return Err(AppError::InvalidState {
            current: format!(
                "timeline_revision_conflict:current={}",
                snapshot.timeline.revision
            ),
            operation: format!("expected={}", options.expected_timeline_revision),
        });
    }
    let order = a4n_db::video::active_timeline_order(&snapshot).map_err(|error| {
        AppError::InvalidState {
            current: error.to_string(),
            operation: "校验插入片段时间线间隙".to_string(),
        }
    })?;
    // 只借用核心 gap 规则校验相邻关系，不写入任何 canonical 项。
    timeline_gap_index(
        &order,
        a4n_core::video_execution::VideoTimelineGap {
            previous_item_id: options.previous_item_id,
            next_item_id: options.next_item_id,
        },
    )
    .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    let reference_ids = options.reference_image_ids.clone();
    if !reference_ids.is_empty() {
        let reference_json = json!(reference_ids);
        service
            .validated_video_reference_images(project_id, chapter_id, Some(&reference_json))
            .await?;
    }
    let neighbor_context = crate::video::timeline::inserted_segment_neighbor_context(
        service,
        project_id,
        chapter_id,
        &snapshot,
        options.previous_item_id,
        options.next_item_id,
    )
    .await?;
    let previous = neighbor_context.previous;
    let next = neighbor_context.next;
    let previous_tail_video_id = previous
        .get("video")
        .and_then(|video| video.get("video_id"))
        .and_then(Value::as_i64);
    let previous_tail_image_id = previous
        .get("video")
        .and_then(|video| video.get("tail_frame_image_id"))
        .and_then(Value::as_i64);
    let guidance_mode = if previous_tail_image_id.is_some() {
        VideoSegmentGuidanceModeV1::Continuity
    } else if !reference_ids.is_empty() {
        VideoSegmentGuidanceModeV1::Keyframes
    } else {
        if !options.allow_without_visual_anchor {
            return Err(AppError::InvalidArgument(
                "插入片段没有可用的前项尾帧或参考图；请补充视觉锚点，或明确允许纯文本生成"
                    .to_string(),
            ));
        }
        VideoSegmentGuidanceModeV1::None
    };
    let workflow_mode = if guidance_mode == VideoSegmentGuidanceModeV1::Continuity {
        VideoSegmentWorkflowModeV1::FirstFrameChain
    } else {
        VideoSegmentWorkflowModeV1::BoundaryFrames
    };
    let continuity_source_video_id =
        continuity_source_for_guidance(guidance_mode, previous_tail_video_id);
    let prompt_output_language =
        PromptOutputLanguage::from_optional_code(options.prompt_output_language.as_deref())?;
    let duration_options = normalized_duration_options(&options);
    let constraints = json!({
        "selected_duration_secs": options.duration_secs,
        "duration_secs_options": duration_options,
        "aspect_ratio": options.aspect_ratio.trim(),
        "guidance_mode": guidance_mode.as_str(),
        "continuity_source_video_id": continuity_source_video_id,
        "continuity_source_tail_frame_image_id": previous_tail_image_id,
        "reference_image_ids": options.reference_image_ids,
        "guidance_reference_image_ids": reference_ids,
        "allow_without_visual_anchor": options.allow_without_visual_anchor,
    });
    let payload = json!({
        "chapter": {
            "project_id": project_id,
            "chapter_id": chapter_id,
            "summary": source_bundle.source.summary,
            "content_sha256": source_content_sha256,
            "content_excerpt": content_excerpt(&source_bundle.source.content),
        },
        "timeline": {
            "timeline_id": options.timeline_id,
            "expected_revision": options.expected_timeline_revision,
            "previous_item_id": options.previous_item_id,
            "next_item_id": options.next_item_id,
        },
        "previous_item": previous,
        "next_item": next,
        "user_prompt": options.user_prompt.trim(),
        "style": options.style.trim(),
        "prompt_output_language": prompt_output_language.as_str(),
        "constraints": constraints,
    });
    let context_sha256 = neighbor_context.sha256;
    let input_fingerprint = json!({
        "schema_version": 1,
        "workflow_kind": crate::video::timeline::INSERTED_SEGMENT_WORKFLOW_KIND,
        "project_id": project_id,
        "chapter_id": chapter_id,
        "source_content_sha256": source_content_sha256,
        "context_sha256": context_sha256,
        "timeline_id": options.timeline_id,
        "timeline_revision": options.expected_timeline_revision,
        "gap": {
            "previous_item_id": options.previous_item_id,
            "next_item_id": options.next_item_id,
        },
        "payload": {
            "user_prompt": options.user_prompt.trim(),
            "style": options.style.trim(),
            "prompt_output_language": prompt_output_language.as_str(),
            "constraints": {
                "selected_duration_secs": options.duration_secs,
                "duration_secs_options": duration_options,
                "aspect_ratio": options.aspect_ratio.trim(),
                "guidance_mode": guidance_mode.as_str(),
                "continuity_source_video_id": continuity_source_video_id,
                "reference_image_ids": options.reference_image_ids,
                "guidance_reference_image_ids": reference_ids,
            },
        },
    });
    let fingerprint = source_text_hash(&serde_json::to_string(&input_fingerprint)?);
    let workflow = service
        .claim_exclusive_workflow_run(ClaimExclusiveWorkflowRunInput {
            run: UpsertWorkflowRunInput {
                business_key: format!(
                    "video_inserted_segment:{project_id}:{chapter_id}:timeline:{}:gap:{}:{}:{}",
                    options.timeline_id,
                    options.expected_timeline_revision,
                    options.previous_item_id.unwrap_or(0),
                    options.next_item_id.unwrap_or(0),
                ),
                workflow_kind: crate::video::timeline::INSERTED_SEGMENT_WORKFLOW_KIND.to_string(),
                project_id: Some(project_id),
                root_project_id: Some(root_project_id),
                source_snapshot_id: None,
                scope_table: Some("chapter_video_timeline".to_string()),
                scope_id: Some(options.timeline_id),
                chapter_id: Some(chapter_id),
                mode: Some(workflow_mode.as_str().to_string()),
                input_fingerprint_json: Some(input_fingerprint.clone()),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                current_step_key: Some(
                    crate::video::timeline::INSERTED_SEGMENT_CHECKPOINT_KEY.to_string(),
                ),
                active_task_id: Some(active_task_id.to_string()),
                can_resume: false,
                resume_block_reason: None,
                error_message: None,
            },
            resume_from_task_id: retry_of_task_id.map(ToString::to_string),
        })
        .await?;
    let workflow_run_id = workflow.id;
    let mut variables = HashMap::new();
    variables.insert("payload".to_string(), payload.clone());
    let manifest = LlmContextLedger::new().into_manifest(
        serde_json::to_string_pretty(&payload).map_err(AppError::Serialization)?,
        Some("chapter_video_inserted_segment".to_string()),
    );
    let mut progress = progress;
    let mut chunks = chunks;
    report_video_prompt_progress(&mut progress, 10, 100, "已准备插入片段前后项和视觉锚点")?;
    let rendered = render_or_restore_json_with_validation_with_context(
        RenderOrRestoreJsonWithValidationInput {
            service,
            workflow_run_id,
            call_key: crate::video::timeline::INSERTED_SEGMENT_CHECKPOINT_KEY.to_string(),
            call_order: 100,
            input_fingerprint_json: input_fingerprint,
            prompt_renderer,
            prompt_name: INSERTED_SEGMENT_PROMPT_NAME,
            project_id,
            variables: &variables,
            context_manifest: manifest,
            temperature: 0.45,
            cancel_token: Some(cancel_token),
            step: VideoPromptStepProgress::new(20, 100, "正在生成插入片段描述"),
            progress: &mut progress,
            chunks: &mut chunks,
            structured_progress: None,
            checkpoint_context: "插入片段草稿断点",
        },
        |raw| {
            let draft = serde_json::from_str::<InsertedSegmentDraft>(raw).map_err(|error| {
                AppError::InvalidArgument(format!("插入片段草稿 JSON 无效: {error}"))
            })?;
            draft.validate().map_err(AppError::InvalidArgument)?;
            Ok(draft)
        },
        |draft| serde_json::to_value(draft).map_err(AppError::Serialization),
    )
    .await;
    let result = match rendered {
        Ok((_trace, _draft)) => {
            service
                .complete_workflow_run_validated(workflow_run_id)
                .await?;
            Ok(GenerationResultRef {
                kind: "inserted_segment_draft".to_string(),
                id: workflow_run_id.to_string(),
                revision: fingerprint,
            })
        }
        Err(error) => {
            let outcome = if matches!(error, AppError::Cancelled) {
                WorkflowRunFinalizeOutcome::Cancelled
            } else {
                WorkflowRunFinalizeOutcome::Failed
            };
            let message = error.to_string();
            service
                .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, Some(&message))
                .await?;
            Err(error)
        }
    }?;
    report_video_prompt_progress(
        &mut progress,
        100,
        100,
        "插入片段草稿已就绪，可提交到时间线",
    )?;
    Ok(result)
}

fn normalized_duration_options(options: &VideoInsertedSegmentGenerationOptions) -> Vec<i32> {
    let mut values = options.duration_secs_options.clone();
    values.sort_unstable();
    values.dedup();
    values
}

fn continuity_source_for_guidance(
    guidance_mode: VideoSegmentGuidanceModeV1,
    previous_video_id: Option<i64>,
) -> Option<i64> {
    (guidance_mode == VideoSegmentGuidanceModeV1::Continuity)
        .then_some(previous_video_id)
        .flatten()
}

fn content_excerpt(content: &str) -> String {
    let trimmed = content.trim();
    let mut chars = trimmed.chars();
    let excerpt = chars.by_ref().take(6000).collect::<String>();
    if chars.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_continuity_guidance_does_not_keep_a_continuity_source() {
        assert_eq!(
            continuity_source_for_guidance(VideoSegmentGuidanceModeV1::Keyframes, Some(9)),
            None
        );
        assert_eq!(
            continuity_source_for_guidance(VideoSegmentGuidanceModeV1::Continuity, Some(9)),
            Some(9)
        );
    }
}
