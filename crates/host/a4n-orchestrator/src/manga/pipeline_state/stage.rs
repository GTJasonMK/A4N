use a4n_error::AppError;

const MANGA_DRAWING_PROMPT_CALL_ORDER_BASE: i32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::manga) enum MangaPipelineStage {
    CharacterBinding,
    SpeechSelection,
    VisualAnchorExtraction,
    ItemExtraction,
    ComicSynthesis,
    PagePlanning,
    DrawingPrompt,
}

impl MangaPipelineStage {
    pub(in crate::manga) const fn key(self) -> &'static str {
        match self {
            Self::CharacterBinding => "character_binding",
            Self::SpeechSelection => "speech_selection",
            Self::VisualAnchorExtraction => "visual_anchor_extraction",
            Self::ItemExtraction => "item_extraction",
            Self::ComicSynthesis => "comic_synthesis",
            Self::PagePlanning => "page_planning",
            Self::DrawingPrompt => "drawing_prompt",
        }
    }

    const fn order(self) -> i32 {
        match self {
            Self::CharacterBinding => 1,
            Self::SpeechSelection => 2,
            Self::VisualAnchorExtraction => 3,
            Self::ItemExtraction => 4,
            Self::ComicSynthesis => 5,
            Self::PagePlanning => 6,
            Self::DrawingPrompt => 7,
        }
    }
}

pub(in crate::manga) const MANGA_PIPELINE_TOTAL_STEPS: i32 =
    MangaPipelineStage::DrawingPrompt.order();

pub(in crate::manga) fn manga_pipeline_stage_status(
    stage: MangaPipelineStage,
) -> (&'static str, i32, &'static str) {
    let message = match stage {
        MangaPipelineStage::CharacterBinding => "正在绑定出场角色身份与外观",
        MangaPipelineStage::SpeechSelection => "正在选择关键对白、想法与旁白",
        MangaPipelineStage::VisualAnchorExtraction => "正在提取场景和视觉锚点",
        MangaPipelineStage::ItemExtraction => "正在抽取漫画关键物品",
        MangaPipelineStage::ComicSynthesis => "正在综合漫画章节节奏",
        MangaPipelineStage::PagePlanning => "正在生成漫画分页规划",
        MangaPipelineStage::DrawingPrompt => "正在生成整页漫画提示词",
    };
    (stage.key(), stage.order(), message)
}

pub(in crate::manga) fn manga_drawing_prompt_call_order(
    page_index: usize,
) -> Result<i32, AppError> {
    let page_index = i32::try_from(page_index).map_err(|_| {
        AppError::InvalidArgument(format!("漫画绘图提示词页索引超出 i32 范围：{page_index}"))
    })?;
    MANGA_DRAWING_PROMPT_CALL_ORDER_BASE
        .checked_add(page_index)
        .ok_or_else(|| AppError::InvalidArgument("漫画绘图提示词 call_order 超出 i32 范围".into()))
}
