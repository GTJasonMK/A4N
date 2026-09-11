//! 漫画导演执行计划领域合同。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod contracts;
mod validation;

pub use contracts::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V2, MANGA_PAGE_PLAN_V3, MangaCameraAngleV2,
    MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2,
    MangaPageGenerationSchemaVersion, MangaPageImportanceV2, MangaPagePacingV2,
    MangaPagePlanSchemaVersion, MangaPagePlanV3, MangaPageRoleV2, MangaPageV3,
    MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2, MangaReadingFlowV2,
    MangaShootingBeatRefV3, MangaShotScaleV2, MangaTextLayerKindV2, MangaTextLayerV2,
    MangaTextPlacementV2,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceAssetKind {
    CharacterPortrait,
    GeneratedImage,
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
    pub bound_asset: Option<MangaBoundReferenceAsset>,
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
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub shooting_script_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageGenerationLayoutV2 {
    pub schema_version: MangaPageGenerationSchemaVersion,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub prompt_fingerprint: Value,
    pub source: MangaPagePromptSourceSummary,
}

impl MangaPageGenerationLayoutV2 {
    pub fn validate(&self, expected_page_number: u32) -> Result<(), MangaPlanValidationError> {
        self.page_plan.validate()?;
        self.drawing_prompt.validate(expected_page_number)?;
        if self.page_plan.page_number != expected_page_number {
            return Err(MangaPlanValidationError {
                path: "page_plan.page_number".to_string(),
                message: format!("必须等于 {expected_page_number}"),
            });
        }
        if self.source.shooting_script_id <= 0 {
            return Err(MangaPlanValidationError {
                path: "source.shooting_script_id".to_string(),
                message: "必须大于 0".to_string(),
            });
        }
        if self.source.shooting_script_revision <= 0 {
            return Err(MangaPlanValidationError {
                path: "source.shooting_script_revision".to_string(),
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
                "source.shooting_script_sha256",
                &self.source.shooting_script_sha256,
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
        }
        Ok(())
    }
}
