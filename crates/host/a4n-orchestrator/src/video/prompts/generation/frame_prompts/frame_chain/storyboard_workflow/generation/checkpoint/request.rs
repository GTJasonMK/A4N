use std::collections::HashMap;

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;
use a4n_video_contract::VideoExecutionUnitKindV2;
use serde_json::{Value, json};

use crate::video::ChapterVideoSourceContext;
use crate::video::prompts::generation::frame_prompts::{
    FramePromptContext, FramePromptGenerationSpec, VideoContextManifestInput, VideoUnitPlan,
    build_storyboard_prompt_user, build_video_context_manifest, video_stage_fingerprint,
};
use crate::video::prompts::json::VideoSegmentPlan;

pub(super) struct StoryboardFrameRequestInput<'a, 'source> {
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub frame_prompt_available_assets: &'a Value,
    pub reference_ids: &'a Value,
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub unit: &'a VideoUnitPlan,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub expected_frame_count: usize,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

pub(super) struct PreparedStoryboardFrameRequest {
    pub frame_number: i32,
    pub duration_secs: i32,
    pub required_aspect_ratio: String,
    pub expected_segment_numbers: Vec<i32>,
    pub context_manifest: LlmContextManifest,
    pub step_key: String,
    pub fingerprint_json: Value,
}

pub(super) async fn prepare_storyboard_frame_request(
    input: StoryboardFrameRequestInput<'_, '_>,
) -> Result<PreparedStoryboardFrameRequest, AppError> {
    let StoryboardFrameRequestInput {
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        frame_prompt_available_assets,
        reference_ids,
        segment_json,
        segment_plan,
        unit,
        style,
        prompt_output_language,
        aspect_ratio,
        expected_frame_count,
        prompt_renderer,
    } = input;
    let frame_number = unit.unit_number;
    let segment = &unit.prompt_segment;
    let (previous_segment, current_segment, next_segment, role, continuity_instruction) =
        match &unit.kind {
            VideoExecutionUnitKindV2::Content(content) => (
                None,
                segment_plan
                    .iter()
                    .find(|segment| segment.segment_number == content.source_segment_number),
                None,
                "content_storyboard",
                "这是当前内容片段的视频单元多格分镜板，用于表达本段核心构图、动作姿态、空间关系和光影，不是首帧或尾帧。",
            ),
            VideoExecutionUnitKindV2::ActionBridge(bridge) => (
                segment_plan
                    .iter()
                    .find(|segment| segment.segment_number == bridge.from_segment_number),
                None,
                segment_plan
                    .iter()
                    .find(|segment| segment.segment_number == bridge.to_segment_number),
                "action_bridge_storyboard",
                "这是拍摄剧本明确要求的动作桥分镜板，只表达 preserved_action 对应的动作接点；不得新增关键剧情、不得重复前后内容单元主动作。",
            ),
        };
    let expected_segment_numbers = unit.source_segment_numbers();
    let selected_spans =
        crate::video::prompts::generation::frame_prompts::checkpoint::source_spans_for_segments(
            source_spans,
            segment_plan
                .iter()
                .filter(|segment| expected_segment_numbers.contains(&segment.segment_number)),
        )?;
    let source_text =
        a4n_domain::source_spans::source_span_context_excerpt(source.content, &selected_spans)
            .map_err(AppError::InvalidArgument)?;
    let scoped_source = ChapterVideoSourceContext {
        content: &source_text,
        summary: source.summary,
        analysis: source.analysis,
    };
    let current_frame = json!({
        "frame_number": frame_number,
        "total_frame_count": expected_frame_count,
        "role": role,
        "video_unit": unit,
        "expected_segment_numbers": expected_segment_numbers,
        "required_duration_secs": unit.duration_secs,
        "required_aspect_ratio": &segment.aspect_ratio,
        "previous_segment": previous_segment,
        "current_segment": current_segment,
        "next_segment": next_segment,
        "reference_slot_instruction": "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。",
        "continuity_instruction": continuity_instruction
    });
    let frame_prompt_context = FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        segment_plan: segment_json,
        available_assets: frame_prompt_available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    };
    let frame_user = build_storyboard_prompt_user(frame_prompt_context, &current_frame).await?;
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables,
        source: &scoped_source,
        prompt_names: &["chapter_video_segment_plan", spec.frame_prompt_name],
        previous_outputs: &[],
        project_context,
        asset_context: None,
        dynamic_user_content: frame_user.clone(),
    })
    .await?;
    let step_key = format!("{}:frame:{frame_number}", spec.frame_prompt_name);
    let fingerprint_json = video_stage_fingerprint(
        spec.frame_prompt_name,
        &context_manifest,
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "source_span_ids": selected_spans.iter().map(|span| &span.id).collect::<Vec<_>>(),
            "frame_number": frame_number,
            "frame_prompt_type": spec.frame_prompt_type,
        }),
    );
    Ok(PreparedStoryboardFrameRequest {
        frame_number,
        duration_secs: unit.duration_secs,
        required_aspect_ratio: segment.aspect_ratio.clone(),
        expected_segment_numbers,
        context_manifest,
        step_key,
        fingerprint_json,
    })
}
