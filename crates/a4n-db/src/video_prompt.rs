//! 视频提示词下游依赖与可变性规则（从 a4n-services 收敛，供 orchestrator facade 使用）。

use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

use crate::video::{self, VideoDbError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VideoPromptDownstreamDependencies {
    pub prompt_ids: Vec<i64>,
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
}

impl VideoPromptDownstreamDependencies {
    pub fn is_empty(&self) -> bool {
        self.prompt_ids.is_empty() && self.image_ids.is_empty() && self.video_ids.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpdateChapterVideoPromptInput {
    pub visual_prompt: Option<String>,
    pub motion_prompt: Option<Option<String>>,
    pub negative_prompt: Option<Option<String>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
    pub metadata_patch: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterVideoPromptSyncPolicy {
    pub preserve_generated_video_prompts: bool,
    pub preserve_frame_prompts_with_generated_images: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedVideoPromptGenerationTrace {
    pub prompt_name: String,
    pub request_json: Value,
    pub rendered_messages_json: Value,
    pub response_text: String,
    pub workflow_run_id: Option<i64>,
    pub usage_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptSyncDraft {
    pub fields: video::ChapterVideoPromptRecordFields,
    pub generation_trace: Option<PreparedVideoPromptGenerationTrace>,
}

type PromptIdentity = (String, String, i32, Option<String>);
type PromptScope = (String, String, Option<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PromptStage {
    SegmentPlan,
    FramePrompt,
    Video,
}

mod dependencies;
pub use dependencies::*;

mod sync;
pub use sync::*;

mod update;
pub use update::*;

#[cfg(test)]
mod tests;
