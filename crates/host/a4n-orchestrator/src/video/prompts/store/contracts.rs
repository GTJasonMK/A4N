use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, UsageInfo};
use a4n_video_contract::VideoSegmentExecutionRecipeV1;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::video) enum PromptWorkflowMode {
    SharedSegmentPlan,
    BoundaryFrames,
    FirstFrameChain,
    ChatVideo,
    Storyboard,
}

impl PromptWorkflowMode {
    pub(in crate::video) fn as_str(self) -> &'static str {
        match self {
            Self::SharedSegmentPlan => chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            Self::BoundaryFrames => chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
            Self::FirstFrameChain => chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
            Self::ChatVideo => chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            Self::Storyboard => chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
        }
    }

    pub(in crate::video) fn from_row(row: &ChapterVideoPromptRecord) -> Result<Self, AppError> {
        match row.workflow_mode.as_str() {
            chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN => Ok(Self::SharedSegmentPlan),
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES => Ok(Self::BoundaryFrames),
            chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN => Ok(Self::FirstFrameChain),
            chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO => Ok(Self::ChatVideo),
            chapter_video_prompt::WORKFLOW_MODE_STORYBOARD => Ok(Self::Storyboard),
            other => Err(AppError::InvalidArgument(format!(
                "章节视频提示词 #{} workflow_mode 无效: {}",
                row.id, other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::video) struct NewChapterVideoPrompt {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: PromptWorkflowMode,
    pub(super) style: Option<String>,
    pub(super) prompt_type: String,
    pub(super) shot_number: i32,
    pub(super) title: Option<String>,
    pub(super) scene_summary: Option<String>,
    pub(super) visual_prompt: String,
    pub(super) motion_prompt: Option<String>,
    pub(super) negative_prompt: Option<String>,
    pub(super) duration_secs: i32,
    pub(super) aspect_ratio: Option<String>,
    pub(super) reference_image_ids_json: Option<Value>,
    pub(super) metadata_json: Option<Value>,
    pub(super) generation_trace: Option<VideoPromptGenerationTrace>,
}

#[derive(Debug, Clone)]
pub(in crate::video) struct VideoPromptGenerationTrace {
    pub(in crate::video) prompt_name: String,
    pub(in crate::video) request: ChatRequest,
    pub(in crate::video) response_text: String,
    pub(in crate::video) workflow_run_id: Option<i64>,
    pub(in crate::video) usage: Option<UsageInfo>,
}

pub(in crate::video) struct NewChapterVideoPromptInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub style: Option<&'a str>,
    pub prompt_type: &'a str,
    pub shot_number: i32,
    pub title: Option<&'a str>,
    pub scene_summary: Option<&'a str>,
    pub visual_prompt: &'a str,
    pub motion_prompt: Option<&'a str>,
    pub negative_prompt: Option<&'a str>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<&'a str>,
    pub reference_image_ids_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl NewChapterVideoPrompt {
    pub(in crate::video) fn with_generation_trace(
        mut self,
        trace: VideoPromptGenerationTrace,
    ) -> Self {
        self.generation_trace = Some(trace);
        self
    }

    pub(in crate::video) fn with_generation_trace_if(
        mut self,
        trace: Option<VideoPromptGenerationTrace>,
    ) -> Self {
        self.generation_trace = trace;
        self
    }

    pub(in crate::video) fn matches_identity(
        &self,
        workflow_mode: PromptWorkflowMode,
        prompt_type: &str,
        shot_number: i32,
    ) -> bool {
        self.workflow_mode == workflow_mode
            && self.prompt_type == prompt_type
            && self.shot_number == shot_number
    }

    pub(in crate::video) fn with_segment_recipe(
        mut self,
        recipe: &VideoSegmentExecutionRecipeV1,
    ) -> Self {
        self.reference_image_ids_json = Some(json!(recipe.provider_reference_image_ids()));
        self
    }
}

pub(in crate::video) fn new_prompt_draft(
    input: NewChapterVideoPromptInput<'_>,
) -> NewChapterVideoPrompt {
    let NewChapterVideoPromptInput {
        project_id,
        chapter_id,
        workflow_mode,
        style,
        prompt_type,
        shot_number,
        title,
        scene_summary,
        visual_prompt,
        motion_prompt,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids_json,
        metadata_json,
    } = input;
    NewChapterVideoPrompt {
        project_id,
        chapter_id,
        workflow_mode,
        style: style.map(ToString::to_string),
        prompt_type: prompt_type.to_string(),
        shot_number,
        title: title.map(ToString::to_string),
        scene_summary: scene_summary.map(ToString::to_string),
        visual_prompt: visual_prompt.to_string(),
        motion_prompt: motion_prompt.map(ToString::to_string),
        negative_prompt: negative_prompt.map(ToString::to_string),
        duration_secs,
        aspect_ratio: aspect_ratio.map(ToString::to_string),
        reference_image_ids_json,
        metadata_json,
        generation_trace: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_video_contract::{
        VIDEO_SEGMENT_EXECUTION_RECIPE_V1, VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1,
    };

    #[test]
    fn video_prompt_draft_does_not_invent_guidance_mode() {
        let draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id: 1,
            chapter_id: 2,
            workflow_mode: PromptWorkflowMode::BoundaryFrames,
            style: None,
            prompt_type: "video_prompt",
            shot_number: 1,
            title: None,
            scene_summary: None,
            visual_prompt: "画面",
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: None,
            reference_image_ids_json: None,
            metadata_json: None,
        });
        assert!(draft.metadata_json.is_none());
    }

    #[test]
    fn system_frames_do_not_freeze_inactive_guidance_inputs() {
        let recipe = VideoSegmentExecutionRecipeV1 {
            schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
            segment_number: 1,
            workflow_mode: VideoSegmentWorkflowModeV1::BoundaryFrames,
            style: "cinematic".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: vec![11],
            video_library_image_ids: Vec::new(),
            guidance_mode: VideoSegmentGuidanceModeV1::SystemFrames,
            guidance_reference_image_ids: vec![22],
            continuity_source_video_id: Some(33),
            reference_media: None,
            adopted_video_id: None,
        };
        let draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id: 1,
            chapter_id: 2,
            workflow_mode: PromptWorkflowMode::BoundaryFrames,
            style: Some("cinematic"),
            prompt_type: "video_prompt",
            shot_number: 1,
            title: None,
            scene_summary: None,
            visual_prompt: "画面",
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9"),
            reference_image_ids_json: None,
            metadata_json: None,
        })
        .with_segment_recipe(&recipe);

        assert_eq!(draft.reference_image_ids_json, Some(json!([11])));
        assert!(draft.metadata_json.is_none());
    }
}
