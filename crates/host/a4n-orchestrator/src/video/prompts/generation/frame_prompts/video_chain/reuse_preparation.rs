use std::collections::{BTreeMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::video_prompt as chapter_video_prompt;
use serde_json::Value;

use super::super::{
    AppError, FramePromptGenerationSpec, GeneratedVideoPromptShot, OrchestratorContext,
    PromptWorkflowRowsContext, VideoSegmentPlan, chat_generated_video_prompt_map,
    ensure_chat_video_prompts_can_generate_without_overwriting,
    ensure_video_prompts_can_regenerate, reusable_generated_video_prompts_from_rows,
};

pub(super) struct VideoPromptReusePreparationInput<'a> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub execution_identity: &'a Value,
    pub segment_storage_prompt_type: &'static str,
    pub prompt_output_language: PromptOutputLanguage,
    pub video_segment_plan: &'a [VideoSegmentPlan],
    pub regenerate_existing: bool,
    pub prompt_batch_size: usize,
}

pub(super) struct PreparedVideoPromptReuse {
    pub generation_limit: usize,
    pub reusable_video_prompts: BTreeMap<i32, GeneratedVideoPromptShot>,
    pub existing_generated_video_prompts: BTreeMap<i32, GeneratedVideoPromptShot>,
    pub matching_generated_video_shots: HashSet<i32>,
}

pub(super) async fn prepare_video_prompt_reuse(
    input: VideoPromptReusePreparationInput<'_>,
) -> Result<PreparedVideoPromptReuse, AppError> {
    let VideoPromptReusePreparationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source_metadata,
        style,
        duration_secs,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint,
        execution_identity,
        segment_storage_prompt_type,
        prompt_output_language,
        video_segment_plan,
        regenerate_existing,
        prompt_batch_size,
    } = input;
    let rows_context = PromptWorkflowRowsContext {
        project_id,
        chapter_id,
        workflow_mode: spec.workflow_mode.as_str(),
        prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO,
        source_metadata,
        style,
        duration_secs,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint: Some(generation_asset_fingerprint),
        source_prompt_type: Some(segment_storage_prompt_type),
        video_execution_identity: execution_identity,
    };
    let matching_video_prompts = service
        .matching_prompt_rows_for_workflow(rows_context, prompt_output_language)
        .await?;
    let is_chat_video = spec.is_chat_video();
    let preflight_rows = &matching_video_prompts;
    if preflight_rows
        .iter()
        .any(|row| row.status == chapter_video_prompt::STATUS_RUNNING)
    {
        return Err(AppError::InvalidArgument(format!(
            "当前章节已有{}正在运行，不能重新生成视频提示词",
            spec.video_prompt_label
        )));
    }
    if regenerate_existing {
        ensure_video_prompts_can_regenerate(preflight_rows, spec.video_prompt_label)?;
    }

    let generation_limit = prompt_batch_size;
    let reusable_video_prompts =
        reusable_generated_video_prompts_from_rows(spec, &matching_video_prompts)?;
    let existing_generated_video_prompt_rows = matching_video_prompts
        .iter()
        .filter(|row| row.generated_video_id.is_some())
        .cloned()
        .collect::<Vec<_>>();
    let existing_generated_video_prompts =
        reusable_generated_video_prompts_from_rows(spec, &existing_generated_video_prompt_rows)?;
    let mut seen_matching_shots = HashSet::new();
    let matching_generated_video_shots = matching_video_prompts
        .iter()
        .filter(|row| seen_matching_shots.insert(row.shot_number))
        .filter(|row| row.generated_video_id.is_some())
        .map(|row| row.shot_number)
        .collect::<HashSet<_>>();
    if is_chat_video {
        let reusable_chat_video_prompts =
            chat_generated_video_prompt_map(&reusable_video_prompts, spec.video_prompt_label)?;
        ensure_chat_video_prompts_can_generate_without_overwriting(
            &matching_video_prompts,
            &matching_video_prompts,
            &reusable_chat_video_prompts,
            video_segment_plan,
            regenerate_existing,
            generation_limit,
        )?;
    }

    Ok(PreparedVideoPromptReuse {
        generation_limit,
        reusable_video_prompts,
        existing_generated_video_prompts,
        matching_generated_video_shots,
    })
}
