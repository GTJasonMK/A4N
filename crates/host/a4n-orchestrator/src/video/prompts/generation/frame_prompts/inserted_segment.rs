use a4n_error::AppError;
use std::collections::HashMap;

use a4n_api_contract::video::VideoInsertedSegmentGenerationOptions;
use a4n_domain::image::validate_reference_image_id_list;
use a4n_domain::video_prompt::chapter_video_prompt;
use a4n_domain::video_timeline::{VideoTimelineGap, timeline_gap_index};
use a4n_domain::{VideoAspectRatio, prompt_language::PromptOutputLanguage, source_text_hash};
use a4n_llm_context::LlmContextLedger;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::GenerationResultRef;
use a4n_video_contract::VideoSegmentGuidanceModeV1;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    workflow::{ClaimExclusiveWorkflowRunInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};
use a4n_domain::collaboration::ProjectAccessRole;

use super::super::super::json::InsertedSegmentDraft;
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
    inserted_segment_gap(options)
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
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
    if !options
        .duration_secs_options
        .contains(&options.duration_secs)
    {
        return Err(AppError::InvalidArgument(
            "插入片段 duration_secs 必须包含在 duration_secs_options 中".to_string(),
        ));
    }
    if options.aspect_ratio.parse::<VideoAspectRatio>().is_err() {
        return Err(AppError::InvalidArgument(format!(
            "插入片段 aspect_ratio 无效：{}，必须是 {}",
            options.aspect_ratio,
            VideoAspectRatio::supported_values()
        )));
    }
    validate_reference_image_id_list(&options.reference_image_ids)
        .map_err(AppError::InvalidArgument)?;
    Ok(())
}

pub(in crate::video) async fn generate_inserted_segment_draft(
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
    let gap = inserted_segment_gap(&options);
    let gap_key = gap
        .canonical_key()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
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
    if snapshot.timeline.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN {
        return Err(AppError::InvalidArgument(
            "插入片段只支持当前共享片段规划时间线".to_string(),
        ));
    }
    if snapshot.timeline.revision != options.expected_timeline_revision {
        return Err(crate::video::timeline::timeline_revision_conflict(
            snapshot.timeline.revision,
            options.expected_timeline_revision,
        ));
    }
    let order = a4n_db::video::active_timeline_order(&snapshot).map_err(|error| {
        AppError::InvalidState {
            current: error.to_string(),
            operation: "校验插入片段时间线间隙".to_string(),
        }
    })?;
    // 只借用核心 gap 规则校验相邻关系，不写入任何 canonical 项。
    timeline_gap_index(&order, gap)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    let reference_ids = &options.reference_image_ids;
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
    let workflow_mode = crate::video::timeline::inserted_segment_workflow_mode(guidance_mode)?;
    let continuity_source_video_id = if guidance_mode == VideoSegmentGuidanceModeV1::Continuity {
        previous_tail_video_id
    } else {
        None
    };
    let prompt_output_language = PromptOutputLanguage::from_code(&options.prompt_output_language)?;
    let mut duration_options = options.duration_secs_options.clone();
    duration_options.sort_unstable();
    duration_options.dedup();
    let constraints = json!({
        "selected_duration_secs": options.duration_secs,
        "duration_secs_options": duration_options,
        "aspect_ratio": &options.aspect_ratio,
        "guidance_mode": guidance_mode.as_str(),
        "continuity_source_video_id": continuity_source_video_id,
        "reference_image_ids": reference_ids,
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
        "prompt_output_language_instruction": prompt_output_language.instruction(),
        "visual_prompt_required_sections": prompt_output_language.visual_sections(),
        "motion_prompt_required_sections": prompt_output_language.motion_sections(),
        "visual_prompt_min_chars": crate::video::MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        "motion_prompt_min_chars": crate::video::MIN_PROVIDER_MOTION_PROMPT_CHARS,
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
            "constraints": constraints,
        },
    });
    let fingerprint = source_text_hash(&serde_json::to_string(&input_fingerprint)?);
    let workflow = service
        .claim_exclusive_workflow_run(ClaimExclusiveWorkflowRunInput {
            run: UpsertWorkflowRunInput {
                business_key: format!(
                    "video_inserted_segment:{project_id}:{chapter_id}:timeline:{}:gap:{}:{gap_key}",
                    options.timeline_id, options.expected_timeline_revision,
                ),
                workflow_kind: crate::video::timeline::INSERTED_SEGMENT_WORKFLOW_KIND.to_string(),
                project_id: Some(project_id),
                root_project_id: Some(root_project_id),
                source_snapshot_id: None,
                scope_table: Some(crate::video::timeline::VIDEO_TIMELINE_SCOPE_TABLE.to_string()),
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
    let manifest = LlmContextLedger::new().into_manifest(
        serde_json::to_string_pretty(&payload).map_err(AppError::Serialization)?,
        Some(INSERTED_SEGMENT_PROMPT_NAME.to_string()),
    );
    let mut variables = HashMap::new();
    variables.insert("payload".to_string(), payload);
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
        |raw| InsertedSegmentDraft::parse_and_validate(raw, prompt_output_language),
        |draft| serde_json::to_value(draft).map_err(AppError::Serialization),
    )
    .await;
    let result = rendered.map(|(_trace, _draft)| GenerationResultRef {
        kind: "inserted_segment_draft".to_string(),
        id: workflow_run_id.to_string(),
        revision: fingerprint,
    });
    let result = service
        .finalize_workflow_result(workflow_run_id, result, "插入片段草稿生成")
        .await?;
    report_video_prompt_progress(
        &mut progress,
        100,
        100,
        "插入片段草稿已就绪，可提交到时间线",
    )?;
    Ok(result)
}

fn inserted_segment_gap(options: &VideoInsertedSegmentGenerationOptions) -> VideoTimelineGap {
    VideoTimelineGap {
        previous_item_id: options.previous_item_id,
        next_item_id: options.next_item_id,
    }
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
