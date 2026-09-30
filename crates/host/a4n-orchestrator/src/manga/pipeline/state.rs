#[cfg(test)]
mod tests;

use a4n_error::AppError;

const MANGA_PAGE_PIPELINE_CALL_ORDER_BASE: i32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::manga) enum MangaPipelineStage {
    CharacterBinding,
    SpeechSelection,
    VisualAnchorExtraction,
    ItemExtraction,
    ComicSynthesis,
    PageSlots,
}

impl MangaPipelineStage {
    pub(in crate::manga) const fn key(self) -> &'static str {
        match self {
            Self::CharacterBinding => "character_binding",
            Self::SpeechSelection => "speech_selection",
            Self::VisualAnchorExtraction => "visual_anchor_extraction",
            Self::ItemExtraction => "item_extraction",
            Self::ComicSynthesis => "comic_synthesis",
            Self::PageSlots => "page_content",
        }
    }

    const fn order(self) -> i32 {
        match self {
            Self::CharacterBinding => 1,
            Self::SpeechSelection => 2,
            Self::VisualAnchorExtraction => 3,
            Self::ItemExtraction => 4,
            Self::ComicSynthesis => 5,
            Self::PageSlots => 6,
        }
    }
}

// The shared setup includes one durable page-slot checkpoint after the five
// chapter-wide analysis calls; page work is counted dynamically per page.
pub(in crate::manga) const MANGA_PIPELINE_TOTAL_STEPS: i32 =
    MangaPipelineStage::PageSlots.order() + 1;

pub(in crate::manga) fn manga_pipeline_stage_status(
    stage: MangaPipelineStage,
) -> (&'static str, i32, &'static str) {
    let message = match stage {
        MangaPipelineStage::CharacterBinding => "正在绑定出场角色身份与外观",
        MangaPipelineStage::SpeechSelection => "正在选择关键对白、想法与旁白",
        MangaPipelineStage::VisualAnchorExtraction => "正在提取场景和视觉锚点",
        MangaPipelineStage::ItemExtraction => "正在抽取漫画关键物品",
        MangaPipelineStage::ComicSynthesis => "正在综合漫画章节节奏",
        MangaPipelineStage::PageSlots => "正在确定漫画页数和每页内容",
    };
    (stage.key(), stage.order(), message)
}

pub(in crate::manga) fn manga_page_pipeline_call_order(
    page_index: usize,
    stage_offset: i32,
) -> Result<i32, AppError> {
    if !(0..=2).contains(&stage_offset) {
        return Err(AppError::InvalidArgument(format!(
            "漫画页面阶段 call_order offset 无效：{stage_offset}"
        )));
    }
    let page_index = i32::try_from(page_index).map_err(|_| {
        AppError::InvalidArgument(format!("漫画页面索引超出 i32 范围：{page_index}"))
    })?;
    MANGA_PAGE_PIPELINE_CALL_ORDER_BASE
        .checked_add(
            page_index.checked_mul(3).ok_or_else(|| {
                AppError::InvalidArgument("漫画页面 call_order 超出 i32 范围".into())
            })?,
        )
        .and_then(|base| base.checked_add(stage_offset))
        .ok_or_else(|| AppError::InvalidArgument("漫画页面 call_order 超出 i32 范围".into()))
}
