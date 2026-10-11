use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::MangaBoundReferenceAsset;
use super::{MangaPageDrawingPromptV2, MangaPageSlotsSchemaVersion, MangaPageV4};
use crate::media_authoring::AdaptationBeatRef;

pub const MANGA_PAGE_PIPELINE_STATE_V1: &str = "manga_page_pipeline_state_v1";
pub const MANGA_PAGE_SLOT_LIMIT: usize = 100;
pub const MANGA_PAGE_REFERENCE_DISCOVERY_V1: &str = "manga_page_reference_discovery_v1";
pub const MANGA_PAGE_REFERENCE_IMAGE_LIMIT: usize = 12;
pub const MANGA_PREPROCESSING_V3: &str = "manga_preprocessing_v3";

pub fn manga_preprocessing_input_fingerprint(
    source_content_sha256: &str,
    chapter_number: i32,
    reference_assets: &Value,
) -> String {
    let inputs = json!({
        "schema_version": MANGA_PREPROCESSING_V3,
        "source_content_sha256": source_content_sha256,
        "chapter_number": chapter_number,
        "reference_assets": reference_assets,
    });
    crate::source_text_hash(&inputs.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaChapterComicSynthesisV1 {
    pub chapter_summary: String,
    pub mood_progression: Vec<String>,
}

impl MangaChapterComicSynthesisV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.chapter_summary.trim().is_empty() {
            return Err("comic_synthesis.chapter_summary 必须是非空字符串".into());
        }
        if !(3..=6).contains(&self.mood_progression.len())
            || self
                .mood_progression
                .iter()
                .any(|phase| phase.trim().is_empty())
        {
            return Err("comic_synthesis.mood_progression 必须包含 3-6 个非空阶段".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPreprocessingV3 {
    pub schema_version: String,
    pub source_events: Value,
    pub characters: Value,
    pub speech: Value,
    pub visual_anchors: Value,
    pub items: Value,
    pub comic_synthesis: MangaChapterComicSynthesisV1,
}

impl MangaPreprocessingV3 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MANGA_PREPROCESSING_V3 {
            return Err(format!("schema_version 必须是 {MANGA_PREPROCESSING_V3}"));
        }
        let events = self
            .source_events
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| "source_events.events 必须是数组".to_string())?;
        if events.is_empty() {
            return Err("source_events.events 至少需要一个事件".into());
        }
        let mut event_ids = std::collections::HashSet::with_capacity(events.len());
        for (index, event) in events.iter().enumerate() {
            let path = format!("source_events.events[{index}]");
            let event_id = event
                .get("event_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{path}.event_id 必须是非空字符串"))?;
            if !event_ids.insert(event_id) {
                return Err(format!("{path}.event_id 不能重复"));
            }
            if event
                .get("description")
                .and_then(Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(format!("{path}.description 必须是非空字符串"));
            }
            if event
                .get("type")
                .and_then(Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
                || event
                    .get("participants")
                    .is_none_or(|value| !value.is_array())
            {
                return Err(format!("{path}.type 和 {path}.participants 无效"));
            }
            let source_span_ids = event
                .get("source_span_ids")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{path}.source_span_ids 必须是数组"))?;
            if source_span_ids.is_empty()
                || source_span_ids
                    .iter()
                    .any(|id| id.as_str().is_none_or(|value| value.trim().is_empty()))
            {
                return Err(format!("{path}.source_span_ids 必须包含来源 ID"));
            }
        }
        if !self.characters.is_object() {
            return Err("characters 必须是对象".into());
        }
        if !self.speech.get("dialogues").is_some_and(Value::is_array)
            || !self.speech.get("narrations").is_some_and(Value::is_array)
        {
            return Err("speech.dialogues 和 speech.narrations 必须是数组".into());
        }
        if !self
            .visual_anchors
            .get("scenes")
            .is_some_and(Value::is_array)
        {
            return Err("visual_anchors.scenes 必须是数组".into());
        }
        if !self.items.get("items").is_some_and(Value::is_array) {
            return Err("items.items 必须是数组".into());
        }
        let event_ids = event_ids
            .into_iter()
            .map(str::to_owned)
            .collect::<std::collections::HashSet<_>>();
        validate_event_references(&self.speech, "dialogues", "source_event_id", &event_ids)?;
        validate_event_references(&self.speech, "narrations", "source_event_id", &event_ids)?;
        validate_event_id_arrays(&self.visual_anchors, "scenes", &event_ids)?;
        validate_event_references(
            &self.items,
            "items",
            "first_appearance_source_event_id",
            &event_ids,
        )?;
        self.comic_synthesis.validate()?;
        Ok(())
    }
}

fn validate_event_references(
    container: &Value,
    array_key: &str,
    reference_key: &str,
    event_ids: &std::collections::HashSet<String>,
) -> Result<(), String> {
    let values = container
        .get(array_key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{array_key} 必须是数组"))?;
    for (index, value) in values.iter().enumerate() {
        let path = format!("{array_key}[{index}].{reference_key}");
        let event_id = value
            .get(reference_key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{path} 必须是非空字符串"))?;
        if !event_ids.contains(event_id) {
            return Err(format!("{path} 引用了不存在的事件：{event_id}"));
        }
    }
    Ok(())
}

fn validate_event_id_arrays(
    container: &Value,
    array_key: &str,
    event_ids: &std::collections::HashSet<String>,
) -> Result<(), String> {
    let values = container
        .get(array_key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{array_key} 必须是数组"))?;
    for (index, value) in values.iter().enumerate() {
        let path = format!("{array_key}[{index}].source_event_ids");
        let references = value
            .get("source_event_ids")
            .and_then(Value::as_array)
            .filter(|references| !references.is_empty())
            .ok_or_else(|| format!("{path} 必须是非空数组"))?;
        for (reference_index, reference) in references.iter().enumerate() {
            let event_id = reference
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{path}[{reference_index}] 必须是非空字符串"))?;
            if !event_ids.contains(event_id) {
                return Err(format!(
                    "{path}[{reference_index}] 引用了不存在的事件：{event_id}"
                ));
            }
        }
    }
    Ok(())
}

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
    ReferenceDiscovery,
    DrawingPrompt,
    Image,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MangaPipelineArtifactStage {
    Preprocessing,
    PageSlots,
    PageContent,
    PageAdaptation,
    Storyboard,
    ReferenceDiscovery,
    ReferenceBindings,
    DrawingPrompt,
    Image,
}

impl MangaPipelineArtifactStage {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Preprocessing,
            Self::PageSlots,
            Self::PageContent,
            Self::PageAdaptation,
            Self::Storyboard,
            Self::ReferenceDiscovery,
            Self::ReferenceBindings,
            Self::DrawingPrompt,
            Self::Image,
        ]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().copied().find(|stage| stage.key() == key)
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Preprocessing => "preprocessing",
            Self::PageSlots => "page_slots",
            Self::PageContent => "page_content",
            Self::PageAdaptation => "adaptation",
            Self::Storyboard => "storyboard",
            Self::ReferenceDiscovery => "reference_discovery",
            Self::ReferenceBindings => "reference_bindings",
            Self::DrawingPrompt => "drawing_prompt",
            Self::Image => "image",
        }
    }

    pub const fn page_scoped(self) -> bool {
        matches!(
            self,
            Self::PageContent
                | Self::PageAdaptation
                | Self::Storyboard
                | Self::ReferenceDiscovery
                | Self::ReferenceBindings
                | Self::DrawingPrompt
                | Self::Image
        )
    }

    pub const fn fingerprint_key(self) -> &'static str {
        match self {
            Self::PageContent => "content",
            _ => self.key(),
        }
    }

    pub fn depends_on(self, upstream: Self) -> bool {
        self.direct_inputs().contains(&upstream)
            || self
                .direct_inputs()
                .iter()
                .any(|direct| direct.depends_on(upstream))
    }

    const fn direct_inputs(self) -> &'static [Self] {
        match self {
            Self::Preprocessing => &[],
            Self::PageSlots => &[Self::Preprocessing],
            Self::PageContent => &[Self::PageSlots],
            Self::PageAdaptation => &[Self::PageContent],
            Self::Storyboard => &[Self::PageContent, Self::PageAdaptation],
            Self::ReferenceDiscovery => &[
                Self::PageContent,
                Self::PageAdaptation,
                Self::Storyboard,
                Self::ReferenceBindings,
            ],
            Self::ReferenceBindings => &[Self::PageContent, Self::Storyboard],
            Self::DrawingPrompt => &[
                Self::PageContent,
                Self::PageAdaptation,
                Self::Storyboard,
                Self::ReferenceDiscovery,
                Self::ReferenceBindings,
            ],
            Self::Image => &[
                Self::ReferenceDiscovery,
                Self::ReferenceBindings,
                Self::DrawingPrompt,
            ],
        }
    }
}

impl MangaPagePipelineStage {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Slots => "slots",
            Self::Content => "content",
            Self::Adaptation => "adaptation",
            Self::Storyboard => "storyboard",
            Self::ReferenceDiscovery => "reference_discovery",
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
    pub fn validate_artifact(&self) -> Result<(), String> {
        if self.visual_intent.trim().is_empty() {
            return Err("visual_intent 不能为空".into());
        }
        if self.beat_refs.is_empty() {
            return Err("beat_refs 至少需要一个节拍".into());
        }
        let mut refs = std::collections::HashSet::with_capacity(self.beat_refs.len());
        for beat_ref in &self.beat_refs {
            beat_ref.validate()?;
            if !refs.insert(beat_ref) {
                return Err("beat_refs 不能包含重复值".into());
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

    pub fn validate(&self, page_content: &MangaPageContentV1) -> Result<(), String> {
        self.validate_artifact()?;
        let content_refs = page_content
            .beat_refs
            .iter()
            .collect::<std::collections::HashSet<_>>();
        if self
            .beat_refs
            .iter()
            .any(|beat_ref| !content_refs.contains(beat_ref))
        {
            return Err("beat_refs 必须来自页面内容节拍".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageImageArtifactV1 {
    /// Legacy image records predate workflow-run provenance.
    pub workflow_run_id: Option<i64>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaPageReferenceTargetKind {
    Character,
    Scene,
    Object,
    Style,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageReferenceImageV1 {
    pub asset: MangaBoundReferenceAsset,
    pub target_kind: MangaPageReferenceTargetKind,
    pub target_name: String,
    pub description: String,
    pub usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageReferenceDiscoveryV1 {
    pub schema_version: String,
    pub references: Vec<MangaPageReferenceImageV1>,
}

impl MangaPageReferenceDiscoveryV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MANGA_PAGE_REFERENCE_DISCOVERY_V1 {
            return Err(format!(
                "schema_version 必须是 {MANGA_PAGE_REFERENCE_DISCOVERY_V1}"
            ));
        }
        if self.references.len() > MANGA_PAGE_REFERENCE_IMAGE_LIMIT {
            return Err(format!(
                "references 不能超过 {MANGA_PAGE_REFERENCE_IMAGE_LIMIT} 项"
            ));
        }
        let mut assets = std::collections::HashSet::with_capacity(self.references.len());
        for (index, reference) in self.references.iter().enumerate() {
            if reference.target_name.trim().is_empty() {
                return Err(format!("references[{index}].target_name 不能为空"));
            }
            if reference.asset.asset_id <= 0 {
                return Err(format!("references[{index}].asset.asset_id 必须大于 0"));
            }
            if reference.asset.image_path.trim().is_empty() {
                return Err(format!("references[{index}].asset.image_path 不能为空"));
            }
            if reference.description.trim().is_empty() {
                return Err(format!("references[{index}].description 不能为空"));
            }
            if reference.usage.trim().is_empty() {
                return Err(format!("references[{index}].usage 不能为空"));
            }
            if !assets.insert((reference.asset.asset_kind, reference.asset.asset_id)) {
                return Err(format!(
                    "references 包含重复素材: {:?}#{}",
                    reference.asset.asset_kind, reference.asset.asset_id
                ));
            }
        }
        Ok(())
    }
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
    #[serde(default)]
    pub reference_discovery: Option<MangaPageReferenceDiscoveryV1>,
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
            adaptation
                .validate_artifact()
                .map_err(|error| format!("pipeline.page_adaptation 无效: {error}"))?;
        }
        if let Some(storyboard) = &self.storyboard {
            if storyboard.page_number != expected_page_number {
                return Err(format!(
                    "pipeline.storyboard.page_number 必须等于 {expected_page_number}"
                ));
            }
            storyboard
                .validate()
                .map_err(|error| format!("pipeline.storyboard 无效: {error}"))?;
        }
        if let Some(discovery) = &self.reference_discovery {
            discovery
                .validate()
                .map_err(|error| format!("pipeline.reference_discovery 无效: {error}"))?;
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
        }
        // Keep the previous image available while an earlier stage is being regenerated.
        if let Some(image) = &self.image
            && (image
                .workflow_run_id
                .is_some_and(|workflow_run_id| workflow_run_id <= 0)
                || image.generated_image_id <= 0
                || image.width <= 0
                || image.height <= 0)
        {
            return Err("pipeline.image 的身份和尺寸必须有效".into());
        }
        if self.stage == MangaPagePipelineStage::Slots {
            return Err("页面记录不能停留在 slots 阶段".into());
        }
        if let Some(error) = &self.error
            && error.stage != self.stage
        {
            return Err("pipeline.error.stage 必须与当前 stage 一致".into());
        }
        Ok(())
    }
}
