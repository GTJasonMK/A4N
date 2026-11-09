//! 漫画导演执行计划领域合同。

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod call_keys;
mod contracts;
mod deletion;
mod validation;

pub use call_keys::{MangaModelCall, manga_deleted_checkpoint_keys};

pub use contracts::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V4, MANGA_PAGE_PIPELINE_STATE_V1,
    MANGA_PAGE_PLAN_V4, MANGA_PAGE_REFERENCE_DISCOVERY_V1, MANGA_PAGE_REFERENCE_IMAGE_LIMIT,
    MANGA_PAGE_REFERENCE_REQUIREMENT_LIMIT, MANGA_PAGE_SLOT_LIMIT, MANGA_PAGE_SLOTS_V1,
    MANGA_PREPROCESSING_V3, MANGA_REFERENCE_DESCRIPTION_MAX_CHARS,
    MANGA_REFERENCE_TARGET_NAME_MAX_CHARS, MANGA_REFERENCE_USAGE_MAX_CHARS, MangaCameraAngleV2,
    MangaChapterComicSynthesisV1, MangaPageAdaptationV1, MangaPageContentDraftV1,
    MangaPageContentV1, MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2,
    MangaPageGenerationSchemaVersion, MangaPageImageArtifactV1, MangaPageImportanceV2,
    MangaPageOutlineItemV1, MangaPageOutlineV1, MangaPagePacingV2, MangaPagePipelineErrorV1,
    MangaPagePipelineStage, MangaPagePipelineStateV1, MangaPagePipelineStatus,
    MangaPagePlanSchemaVersion, MangaPagePlanV4, MangaPageReferenceDiscoveryV1,
    MangaPageReferenceImageV1, MangaPageReferenceNeedV1, MangaPageReferenceTargetKind,
    MangaPageRoleV2, MangaPageSlotV1, MangaPageSlotsSchemaVersion, MangaPageSlotsV1, MangaPageV4,
    MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2, MangaPipelineArtifactStage,
    MangaPreprocessingFacts, MangaPreprocessingSpeech, MangaPreprocessingStep,
    MangaPreprocessingSynthesis, MangaPreprocessingV3, MangaPreprocessingVisuals,
    MangaReadingFlowV2, MangaShotScaleV2, MangaTextLayerKindV2, MangaTextLayerV2,
    MangaTextPlacementV2, manga_preprocessing_input_fingerprint,
};
pub use validation::MangaPlanValidationError;

/// 整页漫画提示词同步草稿，不包含 LLM generation trace 或持久化状态。
#[derive(Debug, Clone)]
pub struct MangaPagePromptSyncDraft {
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub characters_in_frame: Option<Value>,
    pub layout: Option<Value>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MangaPagePromptRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_type: String,
    pub page_number: i32,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub characters_in_frame: Option<Value>,
    pub layout: Option<Value>,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineArtifactRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub stage: MangaPipelineArtifactStage,
    pub page_number: Option<u32>,
    pub input_fingerprint: String,
    pub output_fingerprint: String,
    pub payload: Value,
    pub stale: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Only the artifact fields needed to project a chapter's prompt list.
#[derive(Debug, Clone, PartialEq)]
pub struct MangaPagePromptArtifactMetadata {
    pub stage: MangaPipelineArtifactStage,
    pub page_number: Option<u32>,
    pub stale: bool,
    pub page_slots_payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineArtifactDraft {
    pub project_id: i64,
    pub chapter_id: i64,
    pub stage: MangaPipelineArtifactStage,
    pub page_number: Option<u32>,
    pub input_fingerprint: String,
    pub payload: Value,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceAssetKind {
    CharacterPortrait,
    GeneratedImage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceBindingAction {
    AddUserImage,
    RemoveUserImage,
}

pub trait MangaPortraitReference {
    fn profile_id(&self) -> i64;
    fn portrait_id(&self) -> i64;
    fn character_name(&self) -> &str;
    fn image_path(&self) -> &str;
    fn prompt(&self) -> &str;
    fn portrait_type(&self) -> &str;
    fn expression(&self) -> Option<&str>;
    fn pose(&self) -> Option<&str>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaBoundReferenceAsset {
    pub asset_kind: MangaReferenceAssetKind,
    pub asset_id: i64,
    pub image_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaReferenceBindingSummary {
    pub character_name: String,
    #[serde(default)]
    pub bound_asset: Option<MangaBoundReferenceAsset>,
    #[serde(default)]
    pub user_added_assets: Vec<MangaBoundReferenceAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPagePromptSourceSummary {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub content_sha256: String,
    pub adaptation_plan_id: i64,
    pub adaptation_plan_revision: i32,
    pub adaptation_plan_sha256: String,
}

impl MangaPagePromptSourceSummary {
    pub fn matches_authoring(
        &self,
        source: &crate::media_authoring::AuthoringSourceRef,
        plan: &crate::media_authoring::AuthoringDocumentRef,
    ) -> bool {
        self.root_project_id == source.root_project_id
            && self.source_chapter_id == source.source_chapter_id
            && self.source_snapshot_id == Some(source.source_snapshot_id)
            && self.source_snapshot_chapter_id == Some(source.source_snapshot_chapter_id)
            && self.content_sha256 == source.content_sha256
            && self.source_revision_key
                == format!(
                    "snapshot:{}:chapter:{}:{}",
                    source.source_snapshot_id,
                    source.source_snapshot_chapter_id,
                    source.content_sha256
                )
            && self.adaptation_plan_id == plan.id
            && self.adaptation_plan_revision == plan.revision
            && self.adaptation_plan_sha256 == plan.sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageGenerationLayoutV3 {
    pub schema_version: MangaPageGenerationSchemaVersion,
    pub page_plan: MangaPageV4,
    pub page_plan_ref: Option<crate::media_authoring::AuthoringDocumentRef>,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub prompt_fingerprint: Value,
    pub source: MangaPagePromptSourceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageGenerationLayoutV4 {
    pub schema_version: MangaPageGenerationSchemaVersion,
    pub page_id: String,
    pub page_number: u32,
    pub page_plan_ref: Option<crate::media_authoring::AuthoringDocumentRef>,
    pub pipeline: MangaPagePipelineStateV1,
    #[serde(default)]
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub prompt_fingerprint: Value,
    pub source: MangaPagePromptSourceSummary,
}

impl MangaPageGenerationLayoutV4 {
    pub fn validate(&self, expected_page_number: u32) -> Result<(), MangaPlanValidationError> {
        if self.schema_version != MangaPageGenerationSchemaVersion::V4 {
            return Err(MangaPlanValidationError {
                path: "schema_version".into(),
                message: "必须是 manga_page_generation_v4".into(),
            });
        }
        if self.page_id.trim().is_empty() {
            return Err(MangaPlanValidationError {
                path: "page_id".into(),
                message: "不能为空".into(),
            });
        }
        if self.page_number != expected_page_number {
            return Err(MangaPlanValidationError {
                path: "page_number".into(),
                message: format!("必须等于 {expected_page_number}"),
            });
        }
        if self.page_id != self.pipeline.page_id {
            return Err(MangaPlanValidationError {
                path: "page_id".into(),
                message: "必须与 pipeline.page_id 一致".into(),
            });
        }
        if self.pipeline.source_fingerprint != self.source.content_sha256 {
            return Err(MangaPlanValidationError {
                path: "pipeline.source_fingerprint".into(),
                message: "必须与 source.content_sha256 一致".into(),
            });
        }
        self.pipeline
            .validate(expected_page_number)
            .map_err(|message| MangaPlanValidationError {
                path: "pipeline".into(),
                message,
            })?;
        if let Some(reference) = &self.page_plan_ref
            && (reference.id <= 0
                || reference.revision <= 0
                || reference.sha256.len() != 64
                || !reference
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(MangaPlanValidationError {
                path: "page_plan_ref".into(),
                message: "分页计划版本身份无效".into(),
            });
        }
        validate_page_source_and_bindings(
            &self.source,
            &self.reference_bindings,
            "source",
            "reference_bindings",
        )
    }
}

fn validate_page_source_and_bindings(
    source: &MangaPagePromptSourceSummary,
    reference_bindings: &[MangaReferenceBindingSummary],
    source_path: &str,
    bindings_path: &str,
) -> Result<(), MangaPlanValidationError> {
    if source.adaptation_plan_id <= 0 || source.adaptation_plan_revision <= 0 {
        return Err(MangaPlanValidationError {
            path: source_path.into(),
            message: "改编计划身份必须有效".into(),
        });
    }
    if source.root_project_id <= 0 || source.source_chapter_id <= 0 {
        return Err(MangaPlanValidationError {
            path: source_path.into(),
            message: "项目与章节 ID 必须大于 0".into(),
        });
    }
    if source.source_snapshot_id.is_some() != source.source_snapshot_chapter_id.is_some() {
        return Err(MangaPlanValidationError {
            path: source_path.into(),
            message: "source_snapshot_id 与 source_snapshot_chapter_id 必须同时存在或同时为空"
                .into(),
        });
    }
    for (path, value) in [
        (
            format!("{source_path}.source_revision_key"),
            &source.source_revision_key,
        ),
        (
            format!("{source_path}.content_sha256"),
            &source.content_sha256,
        ),
        (
            format!("{source_path}.adaptation_plan_sha256"),
            &source.adaptation_plan_sha256,
        ),
    ] {
        if value.trim().is_empty() {
            return Err(MangaPlanValidationError {
                path,
                message: "不能为空".into(),
            });
        }
    }
    for (index, binding) in reference_bindings.iter().enumerate() {
        if binding.character_name.trim().is_empty() {
            return Err(MangaPlanValidationError {
                path: format!("{bindings_path}[{index}].character_name"),
                message: "不能为空".into(),
            });
        }
        if let Some(asset) = &binding.bound_asset
            && (asset.asset_id <= 0 || asset.image_path.trim().is_empty())
        {
            return Err(MangaPlanValidationError {
                path: format!("{bindings_path}[{index}].bound_asset"),
                message: "绑定素材身份无效".into(),
            });
        }
        validate_user_reference_assets(binding, &format!("{bindings_path}[{index}]"))?;
    }
    Ok(())
}

impl MangaPageGenerationLayoutV3 {
    pub fn validate(&self, expected_page_number: u32) -> Result<(), MangaPlanValidationError> {
        if let Some(reference) = &self.page_plan_ref
            && (reference.id <= 0
                || reference.revision <= 0
                || reference.sha256.len() != 64
                || !reference
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(MangaPlanValidationError {
                path: "page_plan_ref".into(),
                message: "分页计划版本身份无效".into(),
            });
        }
        self.page_plan.validate()?;
        self.drawing_prompt.validate(expected_page_number)?;
        if self.page_plan.page_number != expected_page_number {
            return Err(MangaPlanValidationError {
                path: "page_plan.page_number".to_string(),
                message: format!("必须等于 {expected_page_number}"),
            });
        }
        if self.source.adaptation_plan_id <= 0 {
            return Err(MangaPlanValidationError {
                path: "source.adaptation_plan_id".to_string(),
                message: "必须大于 0".to_string(),
            });
        }
        if self.source.adaptation_plan_revision <= 0 {
            return Err(MangaPlanValidationError {
                path: "source.adaptation_plan_revision".to_string(),
                message: "必须大于 0".to_string(),
            });
        }
        if self.source.root_project_id <= 0 || self.source.source_chapter_id <= 0 {
            return Err(MangaPlanValidationError {
                path: "source".to_string(),
                message: "项目与章节 ID 必须大于 0".to_string(),
            });
        }
        if self.source.source_snapshot_id.is_some()
            != self.source.source_snapshot_chapter_id.is_some()
        {
            return Err(MangaPlanValidationError {
                path: "source".to_string(),
                message: "source_snapshot_id 与 source_snapshot_chapter_id 必须同时存在或同时为空"
                    .to_string(),
            });
        }
        for (path, value) in [
            (
                "source.source_revision_key",
                &self.source.source_revision_key,
            ),
            ("source.content_sha256", &self.source.content_sha256),
            (
                "source.adaptation_plan_sha256",
                &self.source.adaptation_plan_sha256,
            ),
        ] {
            if value.trim().is_empty() {
                return Err(MangaPlanValidationError {
                    path: path.to_string(),
                    message: "不能为空".to_string(),
                });
            }
        }
        for (index, binding) in self.reference_bindings.iter().enumerate() {
            if binding.character_name.trim().is_empty() {
                return Err(MangaPlanValidationError {
                    path: format!("reference_bindings[{index}].character_name"),
                    message: "不能为空".to_string(),
                });
            }
            if let Some(asset) = &binding.bound_asset {
                if asset.asset_id <= 0 {
                    return Err(MangaPlanValidationError {
                        path: format!("reference_bindings[{index}].bound_asset.asset_id"),
                        message: "必须大于 0".to_string(),
                    });
                }
                if asset.image_path.trim().is_empty() {
                    return Err(MangaPlanValidationError {
                        path: format!("reference_bindings[{index}].bound_asset.image_path"),
                        message: "不能为空".to_string(),
                    });
                }
            }
            validate_user_reference_assets(binding, &format!("reference_bindings[{index}]"))?;
        }
        Ok(())
    }
}

fn validate_user_reference_assets(
    binding: &MangaReferenceBindingSummary,
    path: &str,
) -> Result<(), MangaPlanValidationError> {
    let mut seen = std::collections::HashSet::new();
    if let Some(asset) = &binding.bound_asset {
        seen.insert((asset.asset_kind, asset.asset_id));
    }
    for (index, asset) in binding.user_added_assets.iter().enumerate() {
        if asset.asset_id <= 0 || asset.image_path.trim().is_empty() {
            return Err(MangaPlanValidationError {
                path: format!("{path}.user_added_assets[{index}]"),
                message: "用户参考素材身份无效".into(),
            });
        }
        if !seen.insert((asset.asset_kind, asset.asset_id)) {
            return Err(MangaPlanValidationError {
                path: format!("{path}.user_added_assets[{index}]"),
                message: "用户参考素材不能重复".into(),
            });
        }
    }
    Ok(())
}
