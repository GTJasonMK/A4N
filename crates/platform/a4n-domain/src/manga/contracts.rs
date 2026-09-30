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
    MANGA_PAGE_PIPELINE_STATE_V1, MANGA_PAGE_SLOT_LIMIT, MangaPageAdaptationV1, MangaPageContentV1,
    MangaPageImageArtifactV1, MangaPagePipelineErrorV1, MangaPagePipelineStage,
    MangaPagePipelineStateV1, MangaPagePipelineStatus, MangaPageSlotV1, MangaPageSlotsV1,
};
pub use text::{MangaTextLayerKindV2, MangaTextLayerV2, MangaTextPlacementV2};
pub use versions::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V4, MANGA_PAGE_PLAN_V4,
    MANGA_PAGE_SLOTS_V1, MangaPageDrawingPromptSchemaVersion, MangaPageGenerationSchemaVersion,
    MangaPagePlanSchemaVersion, MangaPageSlotsSchemaVersion,
};

#[cfg(test)]
mod tests;
