use std::collections::HashMap;

use a4n_core::{
    models::chapter::chapter, prompt_language::PromptOutputLanguage,
    video_execution::VideoSegmentExecutionRecipeV1,
};
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{ProgressPhaseKind, ProgressSeverity};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::duration::VideoDurationCapability;

pub const IMPORT_VIDEO_PROMPT_TYPE: &str = "video_prompt";
pub const FRAME_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
pub const FRAME_SEGMENT_PLAN_CONTRACT_VERSION: &str = "video_execution_contract_v5";
pub const BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION: &str = "boundary_frame_segment_local_v4";
pub const FIRST_FRAME_CHAIN_CONTRACT_VERSION: &str = "first_frame_chain_execution_v4";
pub const CHAT_VIDEO_CONTRACT_VERSION: &str = "chat_video_execution_v4";
pub const STORYBOARD_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_unit_v2";
pub const STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_video_v2";
pub const FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT: usize = 1;
pub const VIDEO_PROMPT_CHECKPOINT_STAGE_CODE: &str = "video_prompt_checkpoint";

#[derive(Debug, Clone)]
pub struct VideoPromptProgressUpdate {
    pub stage_code: String,
    pub phase_kind: ProgressPhaseKind,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub unit_label: Option<String>,
    pub item_key: Option<String>,
    pub item_index: Option<u64>,
    pub item_total: Option<u64>,
    pub severity: ProgressSeverity,
    pub draft_text: Option<String>,
}

impl VideoPromptProgressUpdate {
    pub fn progress(current: u32, total: u32, message: impl Into<String>) -> Self {
        let phase_kind = if current == 0 {
            ProgressPhaseKind::Prepare
        } else if current >= total {
            ProgressPhaseKind::Complete
        } else {
            ProgressPhaseKind::Provider
        };
        let severity = if current >= total {
            ProgressSeverity::Success
        } else {
            ProgressSeverity::Info
        };
        Self {
            stage_code: "video_prompt_generation".to_string(),
            phase_kind,
            current,
            total,
            message: message.into(),
            unit_label: Some("步骤".to_string()),
            item_key: None,
            item_index: None,
            item_total: None,
            severity,
            draft_text: None,
        }
    }

    pub fn committed(
        current: u32,
        total: u32,
        item_kind: &str,
        item_number: i32,
        item_position: usize,
        item_total: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            stage_code: "video_prompt_item_committed".to_string(),
            phase_kind: ProgressPhaseKind::Persist,
            current,
            total,
            message: message.into(),
            unit_label: Some("提示词".to_string()),
            item_key: Some(format!("{item_kind}:{item_number}")),
            // 生成流程按“第几项”上报，公共事件合同使用从 0 开始的索引。
            item_index: item_position.checked_sub(1).map(|index| index as u64),
            item_total: Some(item_total as u64),
            severity: ProgressSeverity::Success,
            draft_text: None,
        }
    }

    pub fn checkpoint(
        current: u32,
        total: u32,
        checkpoint_key: impl Into<String>,
        item_index: usize,
        item_total: usize,
        draft_text: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            stage_code: VIDEO_PROMPT_CHECKPOINT_STAGE_CODE.to_string(),
            phase_kind: ProgressPhaseKind::Persist,
            current,
            total,
            message: message.into(),
            unit_label: Some("草稿".to_string()),
            item_key: Some(checkpoint_key.into()),
            item_index: Some(item_index as u64),
            item_total: Some(item_total as u64),
            severity: ProgressSeverity::Success,
            draft_text: Some(draft_text.into()),
        }
    }
}

pub type VideoPromptProgress<'a> =
    &'a mut (dyn FnMut(VideoPromptProgressUpdate) -> Result<(), a4n_core::error::AppError> + Send);
pub type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, PartialEq)]
pub struct ChapterVideoSource {
    pub content: String,
    pub summary: Option<String>,
    pub analysis: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterVideoSource {
    pub chapter: chapter::Model,
    pub source: ChapterVideoSource,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptOptions {
    pub style: String,
    pub prompt_output_language: PromptOutputLanguage,
    pub prompt_batch_size: i32,
    pub segment_plan_prompt_id: Option<i64>,
    pub target_shot_number: Option<i32>,
    /// 服务端从当前共享规划解析出的唯一片段执行方案；客户端不能直接提供。
    pub segment_recipe: Option<VideoSegmentExecutionRecipeV1>,
    pub duration_secs: i32,
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub video_library_image_ids: Vec<i64>,
    pub duration_capability: Option<VideoDurationCapability>,
    pub regenerate_existing: bool,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterVideoPromptStageInput<'a, 'source> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub source: ChapterVideoSourceContext<'source>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterVideoSourceContext<'a> {
    pub content: &'a str,
    pub summary: Option<&'a str>,
    pub analysis: Option<&'a Value>,
}
