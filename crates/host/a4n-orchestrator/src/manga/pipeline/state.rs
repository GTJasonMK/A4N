#[cfg(test)]
mod tests;

use a4n_error::AppError;

const MANGA_PAGE_PIPELINE_CALL_ORDER_BASE: i32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::manga) enum MangaPipelineStage {
    PageSlots,
}

impl MangaPipelineStage {
    pub(in crate::manga) const fn key(self) -> &'static str {
        match self {
            Self::PageSlots => "page_slots",
        }
    }

    const fn order(self) -> i32 {
        match self {
            Self::PageSlots => 1,
        }
    }
}

// The shared setup includes one durable page-slot checkpoint; page work is
// counted dynamically per page after the explicit preprocessing step.
pub(in crate::manga) const MANGA_PIPELINE_TOTAL_STEPS: i32 =
    MangaPipelineStage::PageSlots.order() + 1;

pub(in crate::manga) fn manga_pipeline_stage_status(
    stage: MangaPipelineStage,
) -> (&'static str, i32, &'static str) {
    let message = match stage {
        MangaPipelineStage::PageSlots => "正在确定漫画页数和每页内容",
    };
    (stage.key(), stage.order(), message)
}

pub(in crate::manga) fn manga_page_pipeline_call_order(
    page_index: usize,
    stage_offset: i32,
) -> Result<i32, AppError> {
    if !(0..=3).contains(&stage_offset) {
        return Err(AppError::InvalidArgument(format!(
            "漫画页面阶段 call_order offset 无效：{stage_offset}"
        )));
    }
    let page_index = i32::try_from(page_index).map_err(|_| {
        AppError::InvalidArgument(format!("漫画页面索引超出 i32 范围：{page_index}"))
    })?;
    MANGA_PAGE_PIPELINE_CALL_ORDER_BASE
        .checked_add(
            page_index.checked_mul(4).ok_or_else(|| {
                AppError::InvalidArgument("漫画页面 call_order 超出 i32 范围".into())
            })?,
        )
        .and_then(|base| base.checked_add(stage_offset))
        .ok_or_else(|| AppError::InvalidArgument("漫画页面 call_order 超出 i32 范围".into()))
}
