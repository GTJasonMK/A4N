use std::collections::HashMap;

use crate::models::chapter::chapter;
use crate::models::project::text_anchor;

use super::contracts::AnchorQuoteWithChapter;
use super::lookup::anchor_by_id;

pub fn anchor_quote_with_chapter_number(
    path: &str,
    anchor_id: i64,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<AnchorQuoteWithChapter, String> {
    let chapter_numbers = chapters
        .iter()
        .map(|chapter| (chapter.id, chapter.chapter_number))
        .collect::<HashMap<_, _>>();
    let anchor = anchor_by_id(path, anchor_id, text_anchors)?;
    let source_chapter_id = anchor
        .source_chapter_id
        .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不是章节正文锚点"))?;
    let chapter_number = chapter_numbers
        .get(&source_chapter_id)
        .copied()
        .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不属于本次输入章节"))?;
    Ok(AnchorQuoteWithChapter {
        quote: anchor.quote.clone(),
        chapter_number,
    })
}

pub fn anchor_quotes_with_chapter_numbers(
    path: &str,
    anchor_ids: &[i64],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<AnchorQuoteWithChapter>, String> {
    anchor_ids
        .iter()
        .map(|anchor_id| anchor_quote_with_chapter_number(path, *anchor_id, chapters, text_anchors))
        .collect()
}
