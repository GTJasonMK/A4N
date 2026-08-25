use std::collections::HashMap;

use crate::models::chapter::chapter;
use crate::models::project::text_anchor;

use super::contracts::AnchorQuoteWithChapter;
pub struct AnchorQuoteIndex<'a> {
    chapter_numbers: HashMap<i64, i32>,
    anchors_by_id: HashMap<i64, &'a text_anchor::Model>,
}

impl<'a> AnchorQuoteIndex<'a> {
    pub fn new(chapters: &[chapter::Model], text_anchors: &'a [text_anchor::Model]) -> Self {
        let chapter_numbers = chapters
            .iter()
            .map(|chapter| (chapter.id, chapter.chapter_number))
            .collect();
        let mut anchors_by_id = HashMap::with_capacity(text_anchors.len());
        for anchor in text_anchors {
            anchors_by_id.entry(anchor.id).or_insert(anchor);
        }
        Self {
            chapter_numbers,
            anchors_by_id,
        }
    }

    pub fn quote_with_chapter_number(
        &self,
        path: &str,
        anchor_id: i64,
    ) -> Result<AnchorQuoteWithChapter, String> {
        let anchor = self.anchors_by_id.get(&anchor_id).copied().ok_or_else(|| {
            format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}")
        })?;
        let source_chapter_id = anchor
            .source_chapter_id
            .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不是章节正文锚点"))?;
        let chapter_number = self
            .chapter_numbers
            .get(&source_chapter_id)
            .copied()
            .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不属于本次输入章节"))?;
        Ok(AnchorQuoteWithChapter {
            quote: anchor.quote.clone(),
            chapter_number,
        })
    }
}

pub fn anchor_quote_with_chapter_number(
    path: &str,
    anchor_id: i64,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<AnchorQuoteWithChapter, String> {
    AnchorQuoteIndex::new(chapters, text_anchors).quote_with_chapter_number(path, anchor_id)
}

pub fn anchor_quotes_with_chapter_numbers(
    path: &str,
    anchor_ids: &[i64],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<AnchorQuoteWithChapter>, String> {
    let index = AnchorQuoteIndex::new(chapters, text_anchors);
    anchor_ids
        .iter()
        .map(|anchor_id| index.quote_with_chapter_number(path, *anchor_id))
        .collect()
}
