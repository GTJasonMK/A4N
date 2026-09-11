use std::collections::HashMap;

use a4n_db_models::models::chapter::chapter;
use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_prompt::rendering::PromptRenderer;
use a4n_video_contract::VideoSegmentExecutionRecipeV1;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::generation::step_progress_state;

use a4n_video::VideoDurationCapability;

pub(super) const FRAME_SEGMENT_PLAN_CONTRACT_VERSION: &str = "video_execution_contract_v5";
pub(super) const BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION: &str = "boundary_frame_segment_local_v4";
pub(super) const FIRST_FRAME_CHAIN_CONTRACT_VERSION: &str = "first_frame_chain_execution_v4";
pub(super) const CHAT_VIDEO_CONTRACT_VERSION: &str = "chat_video_execution_v4";
pub(super) const STORYBOARD_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_unit_v2";
pub(super) const STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_video_v2";
pub(super) const FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT: usize = 1;
pub(super) const VIDEO_PROMPT_CHECKPOINT_STAGE_CODE: &str = "video_prompt_checkpoint";

#[derive(Debug, Clone)]
pub(super) struct VideoPromptProgressUpdate {
    pub(super) stage_code: String,
    pub(super) phase_kind: ProgressPhaseKind,
    pub(super) current: u32,
    pub(super) total: u32,
    pub(super) message: String,
    pub(super) unit_label: Option<String>,
    pub(super) item_key: Option<String>,
    pub(super) item_index: Option<u64>,
    pub(super) item_total: Option<u64>,
    pub(super) severity: ProgressSeverity,
    pub(super) draft_text: Option<String>,
}

impl VideoPromptProgressUpdate {
    pub(super) fn progress(current: u32, total: u32, message: impl Into<String>) -> Self {
        let (phase_kind, severity) =
            step_progress_state(current, total, ProgressPhaseKind::ModelWait);
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

    pub(super) fn committed(
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
            phase_kind: ProgressPhaseKind::Save,
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

    pub(super) fn checkpoint(
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
            phase_kind: ProgressPhaseKind::Save,
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

pub(super) type VideoPromptProgress<'a> =
    &'a mut (dyn FnMut(VideoPromptProgressUpdate) -> Result<(), a4n_error::AppError> + Send);
pub(super) type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChapterVideoSource {
    pub(super) content: String,
    pub(super) summary: Option<String>,
    pub(super) analysis: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterVideoSource {
    pub(super) chapter: chapter::Model,
    pub(super) source: ChapterVideoSource,
}

#[derive(Debug, Clone)]
pub(crate) struct ChapterVideoPromptOptions {
    pub(super) style: String,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) prompt_batch_size: i32,
    pub(super) segment_plan_prompt_id: Option<i64>,
    pub(super) target_shot_number: Option<i32>,
    pub(super) target_narrative_unit_index: Option<u32>,
    /// 服务端从当前共享规划解析出的唯一片段执行方案；客户端不能直接提供。
    pub(super) segment_recipe: Option<VideoSegmentExecutionRecipeV1>,
    pub(super) duration_secs: i32,
    pub(super) duration_secs_options: Vec<i32>,
    pub(super) aspect_ratio: String,
    pub(super) reference_image_ids: Vec<i64>,
    pub(super) video_library_image_ids: Vec<i64>,
    pub(super) duration_capability: Option<VideoDurationCapability>,
    pub(super) regenerate_existing: bool,
    pub(super) workflow_run_id: Option<i64>,
}

pub(super) struct ChapterVideoPromptStageInput<'a, 'source> {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: &'a str,
    pub(super) source: ChapterVideoSourceContext<'source>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) options: ChapterVideoPromptOptions,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ChapterVideoSourceContext<'a> {
    pub(super) content: &'a str,
    pub(super) summary: Option<&'a str>,
    pub(super) analysis: Option<&'a Value>,
}
