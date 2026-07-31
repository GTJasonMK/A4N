use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::video_execution::{VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1};
use a4n_llm::provider::{ChatRequest, UsageInfo};
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
            Self::SharedSegmentPlan => "shared_segment_plan",
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::ChatVideo => "chat_video",
            Self::Storyboard => "storyboard",
        }
    }

    pub(in crate::video) fn from_row(row: &chapter_video_prompt::Model) -> Result<Self, AppError> {
        match row.workflow_mode.as_str() {
            "shared_segment_plan" => Ok(Self::SharedSegmentPlan),
            "boundary_frames" => Ok(Self::BoundaryFrames),
            "first_frame_chain" => Ok(Self::FirstFrameChain),
            "chat_video" => Ok(Self::ChatVideo),
            "storyboard" => Ok(Self::Storyboard),
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

    pub(in crate::video) fn with_segment_recipe_metadata(
        mut self,
        segment_plan_prompt_id: i64,
        recipe: &VideoSegmentExecutionRecipeV1,
    ) -> Result<Self, AppError> {
        let metadata = self
            .metadata_json
            .get_or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| {
                AppError::InvalidArgument("视频提示词 metadata_json 必须是对象".to_string())
            })?;
        metadata.insert(
            "segment_plan_prompt_id".to_string(),
            json!(segment_plan_prompt_id),
        );
        metadata.insert("segment_number".to_string(), json!(recipe.segment_number));
        metadata.insert(
            "segment_recipe_fingerprint".to_string(),
            json!(
                recipe
                    .generation_fingerprint()
                    .map_err(AppError::InvalidArgument)?
            ),
        );
        metadata.insert(
            "guidance_mode".to_string(),
            json!(recipe.guidance_mode.as_str()),
        );
        metadata.insert(
            "keyframe_image_ids".to_string(),
            if recipe.guidance_mode == VideoSegmentGuidanceModeV1::Keyframes {
                json!(recipe.guidance_reference_image_ids)
            } else {
                Value::Null
            },
        );
        metadata.insert(
            "continuity_source".to_string(),
            if recipe.guidance_mode == VideoSegmentGuidanceModeV1::Continuity {
                recipe
                    .continuity_source_video_id
                    .map(|video_id| json!({ "video_id": video_id }))
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            },
        );
        self.reference_image_ids_json = Some(json!(recipe.provider_reference_image_ids()));
        Ok(self)
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
    use a4n_core::video_execution::{
        VIDEO_SEGMENT_EXECUTION_RECIPE_V1, VideoSegmentWorkflowModeV1,
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
        .with_segment_recipe_metadata(9, &recipe)
        .unwrap();

        assert_eq!(draft.reference_image_ids_json, Some(json!([11])));
        let metadata = draft.metadata_json.unwrap();
        assert_eq!(metadata["guidance_mode"], "system_frames");
        assert!(metadata["keyframe_image_ids"].is_null());
        assert!(metadata["continuity_source"].is_null());
    }
}
