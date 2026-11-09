#[cfg(test)]
mod tests;

use a4n_error::AppError;

const MANGA_PAGE_PIPELINE_CALL_ORDER_BASE: i32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::manga) enum MangaPagePlanningStep {
    Outline,
    Content(usize),
}

impl MangaPagePlanningStep {
    pub(in crate::manga) fn prompt_name(self) -> &'static str {
        match self {
            Self::Outline => "manga_page_outline",
            Self::Content(_) => "manga_page_content",
        }
    }

    pub(in crate::manga) fn title(self) -> String {
        match self {
            Self::Outline => "轻量页纲".into(),
            Self::Content(index) => format!("第 {} 页剧情内容", index + 1),
        }
    }

    pub(in crate::manga) fn key(self) -> String {
        match self {
            Self::Outline => self.prompt_name().into(),
            Self::Content(index) => format!("manga_page_content.{}", index + 1),
        }
    }

    pub(in crate::manga) fn call_key(self, chapter_id: i64) -> String {
        use a4n_domain::manga::MangaModelCall;
        match self {
            Self::Outline => MangaModelCall::Outline,
            Self::Content(index) => MangaModelCall::PageContent(index as u32 + 1),
        }
        .call_key(chapter_id)
    }

    pub(in crate::manga) fn call_order(self) -> Result<i32, AppError> {
        match self {
            Self::Outline => Ok(6),
            Self::Content(index) if index < a4n_domain::manga::MANGA_PAGE_SLOT_LIMIT => {
                Ok(7 + index as i32)
            }
            Self::Content(_) => Err(AppError::InvalidArgument("漫画页面索引超出页数上限".into())),
        }
    }
}

pub(in crate::manga) fn manga_page_planning_total_steps(
    page_count: usize,
) -> Result<i32, AppError> {
    if !(1..=a4n_domain::manga::MANGA_PAGE_SLOT_LIMIT).contains(&page_count) {
        return Err(AppError::InvalidArgument(
            "漫画页数必须在 1–100 之间".into(),
        ));
    }
    Ok(page_count as i32 + 2)
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
