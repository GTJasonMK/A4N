use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{MangaPageDrawingPromptV2, MangaPageSlotsSchemaVersion, MangaPageV4};
use crate::media_authoring::AdaptationBeatRef;

pub const MANGA_PAGE_PIPELINE_STATE_V1: &str = "manga_page_pipeline_state_v1";
pub const MANGA_PAGE_SLOT_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageSlotsV1 {
    pub schema_version: MangaPageSlotsSchemaVersion,
    pub pages: Vec<MangaPageSlotV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageSlotV1 {
    pub page_number: u32,
    pub content: MangaPageContentV1,
}

impl MangaPageSlotsV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.pages.is_empty() {
            return Err("pages 至少需要一页".into());
        }
        if self.pages.len() > MANGA_PAGE_SLOT_LIMIT {
            return Err(format!("pages 不能超过 {MANGA_PAGE_SLOT_LIMIT} 页"));
        }
        for (index, page) in self.pages.iter().enumerate() {
            let expected =
                u32::try_from(index + 1).map_err(|_| "page_number 超出 u32 范围".to_string())?;
            if page.page_number != expected {
                return Err(format!(
                    "pages[{index}].page_number 必须连续递增，期望 {expected}"
                ));
            }
            page.content
                .validate()
                .map_err(|error| format!("pages[{index}].content: {error}"))?;
            if index > 0 && page.content.entry_state != self.pages[index - 1].content.exit_state {
                return Err(format!(
                    "pages[{index}].content.entry_state 必须与上一页 exit_state 一致"
                ));
            }
        }
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum MangaPagePipelineStage {
    Slots,
    Content,
    Adaptation,
    Storyboard,
    DrawingPrompt,
    Image,
}

impl MangaPagePipelineStage {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Slots => "slots",
            Self::Content => "content",
            Self::Adaptation => "adaptation",
            Self::Storyboard => "storyboard",
            Self::DrawingPrompt => "drawing_prompt",
            Self::Image => "image",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaPagePipelineStatus {
    Pending,
    Running,
    Failed,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageContentV1 {
    pub summary: String,
    pub source_span_ids: Vec<String>,
    pub beat_refs: Vec<AdaptationBeatRef>,
    pub action: String,
    pub reaction: Option<String>,
    pub dialogue_intent: Option<String>,
    pub entry_state: String,
    pub exit_state: String,
}

impl MangaPageContentV1 {
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("summary", &self.summary),
            ("action", &self.action),
            ("entry_state", &self.entry_state),
            ("exit_state", &self.exit_state),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{field} 不能为空"));
            }
        }
        if self.beat_refs.is_empty() {
            return Err("beat_refs 至少需要一个节拍".into());
        }
        let mut span_ids = std::collections::HashSet::with_capacity(self.source_span_ids.len());
        for id in &self.source_span_ids {
            if id.trim().is_empty() || !span_ids.insert(id) {
                return Err("source_span_ids 不能包含空值或重复值".into());
            }
        }
        let mut beat_refs = std::collections::HashSet::with_capacity(self.beat_refs.len());
        for beat_ref in &self.beat_refs {
            beat_ref.validate()?;
            if !beat_refs.insert(beat_ref) {
                return Err("beat_refs 不能包含重复值".into());
            }
        }
        for (field, value) in [
            ("reaction", &self.reaction),
            ("dialogue_intent", &self.dialogue_intent),
        ] {
            if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
                return Err(format!("{field} 不能是空字符串"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageAdaptationV1 {
    pub visual_intent: String,
    pub continuity_requirements: Vec<String>,
    pub retained_details: Vec<String>,
    pub omitted_details: Vec<String>,
    pub beat_refs: Vec<AdaptationBeatRef>,
}

impl MangaPageAdaptationV1 {
    pub fn validate(&self, page_content: &MangaPageContentV1) -> Result<(), String> {
        if self.visual_intent.trim().is_empty() {
            return Err("visual_intent 不能为空".into());
        }
        if self.beat_refs.is_empty() {
            return Err("beat_refs 至少需要一个节拍".into());
        }
        let content_refs = page_content
            .beat_refs
            .iter()
            .collect::<std::collections::HashSet<_>>();
        let mut refs = std::collections::HashSet::with_capacity(self.beat_refs.len());
        for beat_ref in &self.beat_refs {
            beat_ref.validate()?;
            if !content_refs.contains(beat_ref) || !refs.insert(beat_ref) {
                return Err("beat_refs 必须是页面内容节拍的无重复子集".into());
            }
        }
        for (field, values) in [
            ("continuity_requirements", &self.continuity_requirements),
            ("retained_details", &self.retained_details),
            ("omitted_details", &self.omitted_details),
        ] {
            if values.iter().any(|value| value.trim().is_empty()) {
                return Err(format!("{field} 不能包含空字符串"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageImageArtifactV1 {
    pub workflow_run_id: i64,
    pub generated_image_id: i64,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPagePipelineErrorV1 {
    pub stage: MangaPagePipelineStage,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPagePipelineStateV1 {
    pub schema_version: String,
    pub page_id: String,
    pub status: MangaPagePipelineStatus,
    pub stage: MangaPagePipelineStage,
    pub attempt: u32,
    pub source_fingerprint: String,
    pub stage_fingerprints: BTreeMap<String, String>,
    pub page_content: Option<MangaPageContentV1>,
    pub page_adaptation: Option<MangaPageAdaptationV1>,
    pub storyboard: Option<MangaPageV4>,
    pub drawing_prompt: Option<MangaPageDrawingPromptV2>,
    pub image: Option<MangaPageImageArtifactV1>,
    pub error: Option<MangaPagePipelineErrorV1>,
}

impl MangaPagePipelineStateV1 {
    pub fn validate(&self, expected_page_number: u32) -> Result<(), String> {
        if self.schema_version != MANGA_PAGE_PIPELINE_STATE_V1 {
            return Err(format!(
                "pipeline.schema_version 必须是 {MANGA_PAGE_PIPELINE_STATE_V1}"
            ));
        }
        if self.page_id.trim().is_empty() {
            return Err("pipeline.page_id 不能为空".into());
        }
        if self.source_fingerprint.trim().is_empty() {
            return Err("pipeline.source_fingerprint 不能为空".into());
        }
        if let Some(content) = &self.page_content {
            content
                .validate()
                .map_err(|error| format!("pipeline.page_content 无效: {error}"))?;
        }
        if let Some(adaptation) = &self.page_adaptation {
            let content = self
                .page_content
                .as_ref()
                .ok_or_else(|| "pipeline.page_adaptation 需要 page_content".to_string())?;
            adaptation
                .validate(content)
                .map_err(|error| format!("pipeline.page_adaptation 无效: {error}"))?;
        }
        if let Some(storyboard) = &self.storyboard {
            if self.page_adaptation.is_none() {
                return Err("pipeline.storyboard 需要 page_adaptation".into());
            }
            if storyboard.page_number != expected_page_number {
                return Err(format!(
                    "pipeline.storyboard.page_number 必须等于 {expected_page_number}"
                ));
            }
            storyboard
                .validate()
                .map_err(|error| format!("pipeline.storyboard 无效: {error}"))?;
            let content = self
                .page_content
                .as_ref()
                .ok_or_else(|| "pipeline.storyboard 需要 page_content".to_string())?;
            let available_refs = content
                .beat_refs
                .iter()
                .collect::<std::collections::HashSet<_>>();
            if storyboard
                .beat_refs
                .iter()
                .any(|beat_ref| !available_refs.contains(beat_ref))
            {
                return Err("pipeline.storyboard.beat_refs 必须来自 page_content".into());
            }
            if storyboard.entry_state != content.entry_state
                || storyboard.exit_state != content.exit_state
            {
                return Err(
                    "pipeline.storyboard 的 entry_state/exit_state 必须沿用 page_content".into(),
                );
            }
        }
        if let Some(prompt) = &self.drawing_prompt {
            if prompt.page_number != expected_page_number {
                return Err(format!(
                    "pipeline.drawing_prompt.page_number 必须等于 {expected_page_number}"
                ));
            }
            if prompt.image_prompt.trim().is_empty() {
                return Err("pipeline.drawing_prompt.image_prompt 不能为空".into());
            }
            if self.storyboard.is_none() {
                return Err("pipeline.drawing_prompt 需要 storyboard".into());
            }
        }
        if let Some(image) = &self.image {
            if self.stage != MangaPagePipelineStage::Image {
                return Err("pipeline.image 只能出现在 image 阶段".into());
            }
            if self.drawing_prompt.is_none() {
                return Err("pipeline.image 需要 drawing_prompt".into());
            }
            if image.workflow_run_id <= 0
                || image.generated_image_id <= 0
                || image.width <= 0
                || image.height <= 0
            {
                return Err("pipeline.image 的身份和尺寸必须有效".into());
            }
        }
        if self.status == MangaPagePipelineStatus::Completed && self.image.is_none() {
            return Err("completed 页面必须包含 image artifact".into());
        }
        if self.status == MangaPagePipelineStatus::Completed
            && self.stage != MangaPagePipelineStage::Image
        {
            return Err("completed 页面必须停留在 image 阶段".into());
        }
        match self.stage {
            MangaPagePipelineStage::Slots => return Err("页面记录不能停留在 slots 阶段".into()),
            MangaPagePipelineStage::Content => {
                if self.page_content.is_none() {
                    return Err("content 阶段需要 page_content".into());
                }
            }
            MangaPagePipelineStage::Adaptation => {
                if self.page_content.is_none() {
                    return Err("adaptation 阶段需要 page_content".into());
                }
            }
            MangaPagePipelineStage::Storyboard => {
                if self.page_adaptation.is_none() {
                    return Err("storyboard 阶段需要 page_adaptation".into());
                }
            }
            MangaPagePipelineStage::DrawingPrompt => {
                if self.storyboard.is_none() {
                    return Err("drawing_prompt 阶段需要 storyboard".into());
                }
            }
            MangaPagePipelineStage::Image => {
                if self.drawing_prompt.is_none() {
                    return Err("image 阶段需要 drawing_prompt".into());
                }
            }
        }
        if let Some(error) = &self.error
            && error.stage != self.stage
        {
            return Err("pipeline.error.stage 必须与当前 stage 一致".into());
        }
        Ok(())
    }
}
