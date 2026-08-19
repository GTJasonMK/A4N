//! 漫画导演执行计划领域合同。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
