mod assets;
mod contracts;
mod frames;

use std::collections::{HashMap, HashSet};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::Value;

use self::assets::finalize_video_chain_assets;
use self::contracts::{VideoChainAssetFinalizationInput, VideoChainFrameSelectionInput};
use self::frames::select_video_chain_frames;
use super::super::{
    AppError, ChapterVideoPromptOptions, ChapterVideoSourceContext, FramePromptGenerationSpec,
    OrchestratorContext, PreparedVideoPromptInputs, PromptOutputLanguage,
    ReusableSegmentPlanForSpecQuery, VideoFramePrompt, VideoPromptProgress, VideoSegmentPlan,
    VideoUnitPlan, report_video_prompt_progress, reusable_segment_plan_for_spec,
};

pub(super) struct VideoChainPreparationInput<'a, 'source> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: &'a ChapterVideoSourceContext<'source>,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
}

pub(super) struct PreparedVideoChainInputs {
    pub source_metadata: Value,
    pub regenerate_existing: bool,
    pub prompt_batch_size: usize,
    pub style: String,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_ids: Value,
    pub available_assets: Value,
    pub generation_asset_fingerprint: Value,
    pub execution_identity: Value,
    pub vars: HashMap<String, Value>,
    pub allowed_reference_ids: HashSet<i64>,
    pub all_video_units: Vec<VideoUnitPlan>,
    pub frame_json: Value,
    pub frame_by_number: HashMap<i32, VideoFramePrompt>,
    pub video_segment_plan: Vec<VideoSegmentPlan>,
    pub video_segment_json: Value,
    pub video_unit_by_number: HashMap<i32, VideoUnitPlan>,
    pub segment_storage_prompt_type: &'static str,
}

pub(super) async fn prepare_video_chain_inputs(
    input: VideoChainPreparationInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
) -> Result<PreparedVideoChainInputs, AppError> {
    let VideoChainPreparationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        variables,
        options,
    } = input;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        prompt_batch_size,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        video_library_image_ids,
        available_assets,
        vars,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, source, variables, options, false)
        .await?;
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();

    report_video_prompt_progress(progress, 20, 100, spec.read_video_inputs_progress)?;
    let reusable_segment = reusable_segment_plan_for_spec(ReusableSegmentPlanForSpecQuery {
        service,
        project_id,
        chapter_id,
        spec,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: &style,
        duration_secs,
        aspect_ratio: &aspect_ratio,
    })
    .await?
    .ok_or_else(|| AppError::InvalidArgument(spec.missing_video_inputs_message.to_string()))?;
    let selected_frames = select_video_chain_frames(VideoChainFrameSelectionInput {
        service,
        spec,
        project_id,
        chapter_id,
        segment_plan: &reusable_segment.segment_plan,
        duration_secs,
        execution_identity: &reusable_segment.execution_identity,
        prompt_output_language,
        available_assets: &available_assets,
        reference_image_ids: &reference_image_ids,
    })
    .await?;
    let finalized_assets = finalize_video_chain_assets(VideoChainAssetFinalizationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        reference_image_ids,
        video_library_image_ids,
        vars,
        selected_frames,
        progress,
    })
    .await?;

    let mut execution_identity = reusable_segment.execution_identity;
    let identity = execution_identity
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("视频执行身份必须是 JSON 对象".to_string()))?;
    identity.insert(
        "frame_set_sha256".to_string(),
        Value::String(source_content_sha256(&serde_json::to_string(
            &finalized_assets.frame_json,
        )?)),
    );
    identity.insert(
        "prompt_output_language".to_string(),
        Value::String(prompt_output_language.as_str().to_string()),
    );
    identity.insert(
        "generation_asset_fingerprint".to_string(),
        finalized_assets.generation_asset_fingerprint.clone(),
    );

    Ok(PreparedVideoChainInputs {
        source_metadata,
        regenerate_existing,
        prompt_batch_size,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids: finalized_assets.reference_ids,
        available_assets: finalized_assets.available_assets,
        generation_asset_fingerprint: finalized_assets.generation_asset_fingerprint,
        execution_identity,
        vars: finalized_assets.vars,
        allowed_reference_ids: finalized_assets.allowed_reference_ids,
        all_video_units: finalized_assets.all_video_units,
        frame_json: finalized_assets.frame_json,
        frame_by_number: finalized_assets.frame_by_number,
        video_segment_plan: finalized_assets.video_segment_plan,
        video_segment_json: finalized_assets.video_segment_json,
        video_unit_by_number: finalized_assets.video_unit_by_number,
        segment_storage_prompt_type,
    })
}
