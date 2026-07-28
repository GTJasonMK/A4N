use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_llm::provider::{ChatRequest, UsageInfo};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::video) enum PromptWorkflowMode {
    BoundaryFrames,
    FirstFrameChain,
    ChatVideo,
    Storyboard,
}

impl PromptWorkflowMode {
    pub(in crate::video) fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::ChatVideo => "chat_video",
            Self::Storyboard => "storyboard",
        }
    }

    pub(in crate::video) fn from_row(row: &chapter_video_prompt::Model) -> Result<Self, AppError> {
        match row.workflow_mode.as_str() {
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

    pub(in crate::video) fn storage_str(self, prompt_type: &str) -> &'static str {
        if prompt_type == "frame_segment_plan" {
            "shared_segment_plan"
        } else {
            self.as_str()
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
    let metadata_json = if prompt_type == "video_prompt" {
        match metadata_json {
            Some(Value::Object(mut metadata)) => {
                metadata
                    .entry("guidance_mode")
                    .or_insert_with(|| json!("system_frames"));
                Some(Value::Object(metadata))
            }
            None => Some(json!({ "guidance_mode": "system_frames" })),
            malformed => malformed,
        }
    } else {
        metadata_json
    };
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

    #[test]
    fn video_prompt_draft_has_explicit_default_guidance_mode() {
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
        assert_eq!(
            draft
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("guidance_mode")),
            Some(&json!("system_frames"))
        );
    }
}
