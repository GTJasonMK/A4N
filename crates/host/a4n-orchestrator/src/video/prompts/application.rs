use a4n_error::AppError;
use std::collections::{BTreeSet, HashMap};

use a4n_api_contract::video::{
    ChapterVideoPromptSummary, LoadVideoSegmentPlanDraftInput, OptimizeChapterVideoPromptInput,
    UpdateChapterVideoPromptInput, UpdateChapterVideoSegmentPlanInput,
    UpdateVideoSegmentPlanDraftRecipeInput, VideoSegmentPlanDraftSummary,
};
use a4n_api_contract::video::{VideoPromptDownstreamCleanupPort, VideoPromptGenerationOptions};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptPatch;
use a4n_domain::{DownstreamConflictStrategy, source_text_hash as source_content_sha256};
use a4n_prompt::rendering::PromptRenderer;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_contract::{
    VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2, VideoSegmentWorkflowModeV1,
    VideoShootingScriptIdentityV1, resolve_video_segment_number,
    video_segment_recipes_for_plan_from_metadata,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    workflow::{
        WorkflowRunSummary, workflow_run_is_running_or_resumable, workflow_run_summary_to_api,
    },
};
use a4n_domain::collaboration::ProjectAccessRole;

use super::super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSource,
    ChapterVideoSourceContext, VideoPromptChunk, VideoPromptDownstreamDependencies,
    VideoPromptProgress, VideoPromptProgressUpdate, duration_capability_for_provider_config,
    load_video_segment_plan_draft_checkpoints, load_video_segment_plan_draft_recipe_state,
    update_video_segment_plan_draft_recipe_checkpoint,
    validate_video_duration_for_provider_config_secs, video_db_error,
};
use super::context::validate_prompt_options_with_reference_limit;
use super::generation::{video_execution_chain_identity, video_projection_from_shooting_script};
use super::json::{
    validate_segment_plan, validate_segment_plan_against_shooting_script_projection,
};
use super::optimization::VideoPromptOptimizationInput;

const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoPromptInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(skip)]
    pub active_task_id: Option<String>,
    #[serde(skip)]
    pub root_project_id: Option<i64>,
    #[serde(skip)]
    pub retry_of_task_id: Option<String>,
    pub options: VideoPromptGenerationOptions,
}

pub struct VideoPromptApplicationContext<'a> {
    pub runtime_video_config: Option<&'a VideoProviderConfig>,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

#[derive(Clone, Copy)]
enum PromptGenerationStage {
    Frame,
    Segment,
    Video,
}

struct GenerationSpec {
    operation: &'static str,
    empty_content_message: &'static str,
    workflow_kind: &'static str,
    workflow_step_key: &'static str,
}

struct PreparedGeneration {
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    source: ChapterVideoSource,
    variables: HashMap<String, Value>,
    options: ChapterVideoPromptOptions,
    workflow_run_id: i64,
}

struct VideoPromptMutationPlan {
    allow_downstream: bool,
    delete_dependencies: Option<VideoPromptDownstreamDependencies>,
}

struct VideoSegmentPlanDraftContext {
    run: WorkflowRunSummary,
    style: String,
}

mod execution;

fn generation_spec(stage: PromptGenerationStage) -> GenerationSpec {
    match stage {
        PromptGenerationStage::Frame => GenerationSpec {
            operation: "生成边界帧提示词",
            empty_content_message: "章节正文为空，无法生成边界帧提示词",
            workflow_kind: "video_frame_prompts",
            workflow_step_key: "frame_prompts",
        },
        PromptGenerationStage::Segment => GenerationSpec {
            operation: "生成片段规划",
            empty_content_message: "章节正文为空，无法生成片段规划",
            workflow_kind: "video_segment_plan",
            workflow_step_key: "segment_plan",
        },
        PromptGenerationStage::Video => GenerationSpec {
            operation: "生成视频提示词",
            empty_content_message: "章节正文为空，无法生成视频提示词",
            workflow_kind: "video_prompts",
            workflow_step_key: "video_prompts",
        },
    }
}

fn generation_prepare_message(workflow_mode: &str, stage: PromptGenerationStage) -> &'static str {
    if !matches!(stage, PromptGenerationStage::Video) {
        return "正在准备章节正文、素材和时长档位";
    }
    match workflow_mode {
        chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN => "正在准备章节正文、首帧和素材",
        chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO => "正在准备章节正文和 Chat 上传素材",
        chapter_video_prompt::WORKFLOW_MODE_STORYBOARD => "正在准备章节正文、分镜图和素材",
        _ => "正在准备章节正文、边界帧和素材",
    }
}

fn ensure_video_workflow_mode(input: &VideoPromptGenerationOptions) -> Result<String, AppError> {
    let mode = input
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("视频提示词生成缺少 workflow_mode".to_string()))?;
    if !chapter_video_prompt::WORKFLOW_MODES.contains(&mode) {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词生成 workflow_mode 无效: {mode}"
        )));
    }
    Ok(mode.to_string())
}

fn ensure_video_workflow_mode_for_stage(
    input: &VideoPromptGenerationOptions,
    stage: PromptGenerationStage,
) -> Result<String, AppError> {
    let mode = ensure_video_workflow_mode(input)?;
    match stage {
        PromptGenerationStage::Segment
            if mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN =>
        {
            Err(AppError::InvalidArgument(
                "共享片段规划 workflow_mode 必须为 shared_segment_plan".to_string(),
            ))
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN =>
        {
            Err(AppError::InvalidArgument(
                "片段卡片生成必须使用卡片执行模式，不能使用 shared_segment_plan".to_string(),
            ))
        }
        _ => Ok(mode),
    }
}

fn validate_video_prompt_generation_options_for_stage(
    input: &VideoPromptGenerationOptions,
    stage: PromptGenerationStage,
) -> Result<(), AppError> {
    let workflow_mode = ensure_video_workflow_mode_for_stage(input, stage)?;
    match stage {
        PromptGenerationStage::Segment
            if input.segment_plan_prompt_id.is_some() || input.target_segment.is_some() =>
        {
            return Err(AppError::InvalidArgument(
                "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
            ));
        }
        PromptGenerationStage::Frame | PromptGenerationStage::Video
            if input.segment_plan_prompt_id.is_none() || input.target_segment.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "片段卡片生成必须提供 segment_plan_prompt_id 和目标片段".to_string(),
            ));
        }
        _ => {}
    }
    validate_generate_prompt_options(input.clone(), &workflow_mode).map(|_| ())
}

pub(crate) fn validate_video_segment_plan_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Segment)
}

pub(crate) fn validate_video_frame_prompt_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Frame)
}

pub(crate) fn validate_video_final_prompt_generation_options(
    input: &VideoPromptGenerationOptions,
) -> Result<(), AppError> {
    validate_video_prompt_generation_options_for_stage(input, PromptGenerationStage::Video)
}

fn validate_generate_prompt_options(
    input: VideoPromptGenerationOptions,
    workflow_mode: &str,
) -> Result<ChapterVideoPromptOptions, AppError> {
    let style = input.style.trim().to_string();
    if style.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词生成缺少 style".to_string(),
        ));
    }
    let prompt_output_language = PromptOutputLanguage::from_code(&input.prompt_output_language)?;
    let reference_limit = if workflow_mode == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO {
        MAX_CHAT_REFERENCE_IMAGE_IDS
    } else {
        MAX_NON_CHAT_REFERENCE_IMAGE_IDS
    };
    validate_prompt_options_with_reference_limit(
        ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            prompt_batch_size: input.prompt_batch_size,
            segment_plan_prompt_id: input.segment_plan_prompt_id,
            target_shot_number: None,
            target_narrative_unit_index: input.target_narrative_unit_index,
            segment_recipe: None,
            duration_secs: input.duration_secs,
            duration_secs_options: input.duration_secs_options,
            aspect_ratio: input.aspect_ratio,
            reference_image_ids: input.reference_image_ids,
            video_library_image_ids: input.video_library_image_ids,
            duration_capability: None,
            regenerate_existing: input.regenerate_existing,
            workflow_run_id: None,
        },
        reference_limit,
    )
}

fn apply_duration_capability(
    config: &VideoProviderConfig,
    options: &mut ChapterVideoPromptOptions,
) -> Result<(), AppError> {
    validate_video_duration_for_provider_config_secs(config, options.duration_secs)
        .map_err(AppError::InvalidArgument)?;
    let capability =
        duration_capability_for_provider_config(config).map_err(AppError::InvalidArgument)?;
    capability.validate_duration_options(&options.duration_secs_options)?;
    options.duration_capability = Some(capability);
    Ok(())
}

fn build_video_prompt_vars(
    project_id: i64,
    chapter_id: i64,
    chapter: &ChapterRecord,
    source: &ChapterVideoSource,
    options: &ChapterVideoPromptOptions,
) -> Result<HashMap<String, Value>, AppError> {
    let mut variables = HashMap::new();
    variables.insert("project_id".into(), json!(project_id));
    variables.insert("chapter_id".into(), json!(chapter_id));
    variables.insert("chapter_number".into(), json!(chapter.chapter_number));
    variables.insert(
        "chapter_title".into(),
        json!(required_chapter_title(
            chapter.title.as_deref(),
            chapter.chapter_number,
        )?),
    );
    variables.insert("chapter_status".into(), json!(&chapter.status));
    variables.insert(
        "source_content_sha256".into(),
        json!(source_content_sha256(&source.content)),
    );
    if let Some(summary) = source
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        variables.insert("chapter_summary".into(), json!(summary));
    }
    if let Some(analysis) = source.analysis.as_ref() {
        variables.insert("chapter_analysis".into(), analysis.clone());
        if let Some(compressed) = analysis
            .get("summaries")
            .and_then(|value| value.get("compressed"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            variables.insert("chapter_analysis_summary".into(), json!(compressed));
        }
    }
    variables.insert("style".into(), json!(&options.style));
    variables.insert(
        "prompt_output_language".into(),
        json!(options.prompt_output_language.as_str()),
    );
    variables.insert("duration_secs".into(), json!(options.duration_secs));
    variables.insert(
        "duration_secs_options".into(),
        json!(&options.duration_secs_options),
    );
    if let Some(capability) = options.duration_capability.as_ref() {
        variables.insert(
            "video_duration_capability".into(),
            capability.to_prompt_json(),
        );
    }
    variables.insert("aspect_ratio".into(), json!(&options.aspect_ratio));
    variables.insert(
        "reference_image_ids".into(),
        json!(&options.reference_image_ids),
    );
    variables.insert(
        "video_library_image_ids".into(),
        json!(&options.video_library_image_ids),
    );
    Ok(variables)
}

fn required_chapter_title(title: Option<&str>, chapter_number: i32) -> Result<&str, AppError> {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "第 {chapter_number} 章缺少标题，无法生成视频提示词"
            ))
        })
}

fn video_prompt_source_content_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata?
        .get("video_execution_identity")?
        .get("source_content_sha256")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn mutation_plan_from_dependencies(
    dependencies: VideoPromptDownstreamDependencies,
    strategy: DownstreamConflictStrategy,
    operation: &str,
    subject: &str,
) -> Result<VideoPromptMutationPlan, AppError> {
    if dependencies.is_empty() {
        return Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: None,
        });
    }
    match strategy {
        DownstreamConflictStrategy::Reject => Err(AppError::InvalidState {
            current: format!(
                "{subject}已有 {} 个下游派生产物",
                dependencies.prompt_ids.len()
                    + dependencies.image_ids.len()
                    + dependencies.video_ids.len()
            ),
            operation: format!("{operation}；请选择删除下游后继续，或保留下游继续"),
        }),
        DownstreamConflictStrategy::KeepDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: true,
            delete_dependencies: None,
        }),
        DownstreamConflictStrategy::DeleteDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: Some(dependencies),
        }),
    }
}

fn validate_segment_plan_edit_identity(
    current: &[VideoSegmentPlanV2],
    requested: &[VideoSegmentPlanV2],
) -> Result<(), AppError> {
    if current.len() != requested.len() || requested.is_empty() {
        return Err(AppError::InvalidArgument(
            "片段规划编辑不能新增、删除或重排片段".to_string(),
        ));
    }
    for (index, (before, after)) in current.iter().zip(requested).enumerate() {
        let identity_unchanged = before.schema_version == after.schema_version
            && before.segment_number == after.segment_number
            && before.source_shot_id == after.source_shot_id
            && before.segment_index_in_shot == after.segment_index_in_shot
            && before.shot_segment_count == after.shot_segment_count
            && before.source_span_ids == after.source_span_ids
            && before.duration_secs == after.duration_secs
            && before.aspect_ratio == after.aspect_ratio
            && before.reference_image_ids == after.reference_image_ids
            && before.director_constraints == after.director_constraints;
        if !identity_unchanged {
            return Err(AppError::InvalidArgument(format!(
                "片段规划第 {} 项包含不可编辑的来源或派生字段",
                index + 1
            )));
        }
    }
    Ok(())
}

fn report_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(VideoPromptProgressUpdate::progress(current, total, message))?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), AppError> {
    if cancel_token.is_cancelled() {
        Err(AppError::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment_plan_options(workflow_mode: &str) -> VideoPromptGenerationOptions {
        VideoPromptGenerationOptions {
            workflow_mode: Some(workflow_mode.to_string()),
            segment_plan_prompt_id: None,
            style: "cinematic".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            prompt_batch_size: 1,
            target_segment: None,
            target_narrative_unit_index: None,
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
            video_library_image_ids: Vec::new(),
            regenerate_existing: false,
        }
    }

    #[test]
    fn segment_plan_accepts_only_shared_mode_before_task_creation() {
        assert!(
            validate_video_segment_plan_generation_options(&segment_plan_options(
                chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            ))
            .is_ok()
        );
        let error = validate_video_segment_plan_generation_options(&segment_plan_options(
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
        ))
        .unwrap_err();
        assert!(error.to_string().contains("必须为 shared_segment_plan"));
    }

    #[test]
    fn stage_validation_accepts_stable_card_target_and_rejects_it_for_segment_plan() {
        let mut options = segment_plan_options(chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES);
        options.segment_plan_prompt_id = Some(7);
        options.target_segment = Some(a4n_api_contract::video::VideoSegmentTarget {
            source_shot_id: "shot-1".to_string(),
            segment_index_in_shot: 1,
        });

        assert!(validate_video_frame_prompt_generation_options(&options).is_ok());
        assert!(validate_video_final_prompt_generation_options(&options).is_ok());

        options.workflow_mode =
            Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN.to_string());
        options.segment_plan_prompt_id = None;
        let error = validate_video_segment_plan_generation_options(&options).unwrap_err();
        assert!(error.to_string().contains("不能指定片段规划 ID 或目标片段"));
    }

    #[test]
    fn mutation_strategy_keeps_one_source_of_truth() {
        let dependencies = VideoPromptDownstreamDependencies {
            prompt_ids: vec![1],
            image_ids: vec![2],
            video_ids: vec![3],
        };
        let plan = mutation_plan_from_dependencies(
            dependencies,
            DownstreamConflictStrategy::DeleteDownstream,
            "更新视频提示词",
            "该视频提示词",
        )
        .unwrap();
        assert!(!plan.allow_downstream);
        assert_eq!(plan.delete_dependencies.unwrap().video_ids, vec![3]);
    }

    #[test]
    fn shared_plan_source_hash_requires_execution_identity() {
        let current = json!({
            "source": { "content_sha256": "legacy" },
            "video_execution_identity": { "source_content_sha256": "current" },
        });
        assert_eq!(
            video_prompt_source_content_hash(Some(&current)),
            Some("current")
        );

        let legacy = json!({
            "source": { "content_sha256": "legacy" },
        });
        assert_eq!(video_prompt_source_content_hash(Some(&legacy)), None);
    }

    fn editable_segment_fixture() -> VideoSegmentPlanV2 {
        serde_json::from_value(json!({
            "director_constraints": null,
            "schema_version": "video_segment_plan_v2",
            "segment_number": 1,
            "source_shot_id": "shot-1",
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": ["beat-1"],
            "title": "片段",
            "scene_summary": "场景",
            "blocking_summary": "走位",
            "spatial_anchors": [{"anchor": "门口", "usage": "建立空间"}],
            "emotion_action_goals": [],
            "video_intent": {
                "hook": "钩子",
                "audience_effect": "效果",
                "rhythm": "节奏",
                "visual_priority": "重点"
            },
            "emotional_arc": {
                "start": "开始",
                "turn": "转折",
                "end": "结束",
                "function": "功能"
            },
            "performance_plan": {
                "movement_path": "路径",
                "body_performance": "身体",
                "gaze_and_hands": "视线",
                "pause_or_reaction": "反应",
                "camera_relationship": "关系",
                "time_blocks": [],
                "action_script": []
            },
            "material_coverage": {
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "风险"
            },
            "transition_to_next": null,
            "source_span_ids": ["span-1"],
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        }))
        .unwrap()
    }

    #[test]
    fn segment_plan_edit_keeps_content_editable_but_identity_read_only() {
        let before = editable_segment_fixture();
        let mut edited = before.clone();
        edited.title = "新的片段".to_string();
        assert!(
            validate_segment_plan_edit_identity(std::slice::from_ref(&before), &[edited]).is_ok()
        );

        let mut identity_change = before.clone();
        identity_change.source_shot_id = "shot-2".to_string();
        let error = validate_segment_plan_edit_identity(&[before], &[identity_change]).unwrap_err();
        assert!(error.to_string().contains("不可编辑"));
    }
}
