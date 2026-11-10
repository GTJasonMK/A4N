//! 漫画计划与整页绘图提示词合同。

mod drawing;
mod page;
mod panel;
mod pipeline;
mod text;
mod versions;

pub use drawing::MangaPageDrawingPromptV2;
pub use page::{
    MangaPageImportanceV2, MangaPagePacingV2, MangaPagePlanV4, MangaPageRoleV2, MangaPageV4,
    MangaReadingFlowV2,
};
pub use panel::{
    MangaCameraAngleV2, MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2, MangaShotScaleV2,
};
pub use pipeline::{
    MANGA_PAGE_PIPELINE_STATE_V1, MANGA_PAGE_REFERENCE_DISCOVERY_V1,
    MANGA_PAGE_REFERENCE_IMAGE_LIMIT, MANGA_PAGE_REFERENCE_REQUIREMENT_LIMIT,
    MANGA_PAGE_SLOT_LIMIT, MANGA_PREPROCESSING_V3, MANGA_REFERENCE_DESCRIPTION_MAX_CHARS,
    MANGA_REFERENCE_IMAGES_PER_TARGET, MANGA_REFERENCE_TARGET_NAME_MAX_CHARS,
    MANGA_REFERENCE_USAGE_MAX_CHARS, MangaChapterComicSynthesisV1, MangaPageAdaptationV1,
    MangaPageContentDraftV1, MangaPageContentV1, MangaPageImageArtifactV1, MangaPageOutlineItemV1,
    MangaPageOutlineV1, MangaPagePipelineErrorV1, MangaPagePipelineStage, MangaPagePipelineStateV1,
    MangaPagePipelineStatus, MangaPageReferenceDiscoveryV1, MangaPageReferenceImageV1,
    MangaPageReferenceNeedV1, MangaPageReferenceTargetKind, MangaPageSlotV1, MangaPageSlotsV1,
    MangaPipelineArtifactStage, MangaPreprocessingFacts, MangaPreprocessingSpeech,
    MangaPreprocessingStep, MangaPreprocessingSynthesis, MangaPreprocessingV3,
    MangaPreprocessingVisuals, manga_preprocessing_input_fingerprint,
};
pub use text::{MangaTextLayerKindV2, MangaTextLayerV2, MangaTextPlacementV2};
pub use versions::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V4, MANGA_PAGE_PLAN_V4,
    MANGA_PAGE_SLOTS_V1, MangaPageDrawingPromptSchemaVersion, MangaPageGenerationSchemaVersion,
    MangaPagePlanSchemaVersion, MangaPageSlotsSchemaVersion,
};

#[cfg(test)]
mod tests;
