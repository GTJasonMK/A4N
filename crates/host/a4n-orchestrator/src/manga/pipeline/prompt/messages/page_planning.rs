use a4n_domain::{manga::MangaPageOutlineV1, source_spans::SourceSpan};
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::{Value, json};

use super::{
    common::build_manga_step_user_message,
    inputs::{manga_page_outline_input, manga_page_source_input},
};
use crate::source_markers::SourceMarkerMap;
use crate::{
    manga::adaptation_projection::manga_page_beats_source, media_authoring::PreparedAdaptation,
};

pub(in crate::manga) fn manga_page_outline_user_message(
    extraction_summary: Value,
    adaptation_plan: Value,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_page_outline",
        "严格按已采用页数分配每页的节拍、来源和边界状态，只输出轻量页纲",
        source_markers.encode_input(&manga_page_outline_input(
            extraction_summary,
            adaptation_plan,
        )?)?,
        json!({"format":"json_object", "prompt_name":"manga_page_outline", "schema_version":"manga_page_outline_v1"}),
    )
}

pub(in crate::manga) struct MangaPageContentMessages<'a> {
    pub outline: &'a MangaPageOutlineV1,
    pub extraction_summary: &'a Value,
    pub adaptation: &'a PreparedAdaptation,
    pub source_spans: &'a [SourceSpan],
    pub chapter_content: &'a str,
    pub source_markers: &'a SourceMarkerMap,
}

impl MangaPageContentMessages<'_> {
    pub(in crate::manga) fn build(&self, index: usize) -> Result<LlmContextManifest, AppError> {
        let page = self
            .outline
            .pages
            .get(index)
            .ok_or_else(|| AppError::InvalidArgument(format!("页纲索引不存在：{index}")))?;
        let input = json!({
            "page_number":page.page_number, "total_pages":self.outline.pages.len(),
            "page_outline":page,
            "entry_state":self.outline.entry_state(index).map_err(AppError::InvalidArgument)?,
            "previous_page_outline":index.checked_sub(1).and_then(|previous| self.outline.pages.get(previous)),
            "next_page_outline":self.outline.pages.get(index+1),
            "adaptation_plan":manga_page_beats_source(self.adaptation, &page.beat_refs),
            "chapter_summary":self.extraction_summary.pointer("/comic_synthesis/chapter_summary"),
            "source_context":manga_page_source_input(&page.source_span_ids, self.extraction_summary, self.source_spans, self.chapter_content, self.source_markers)?,
        });
        build_manga_step_user_message(
            "manga_page_content",
            &format!(
                "只生成第 {}/{} 页承载的剧情内容，遵守页纲的分配与起止状态",
                page.page_number,
                self.outline.pages.len()
            ),
            self.source_markers.encode_input(&input)?,
            json!({"format":"json_object", "prompt_name":"manga_page_content", "page_number":page.page_number}),
        )
    }
}
