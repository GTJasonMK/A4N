use super::types::{SourceChapter, SourceTextAnchor};
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorQuoteWithChapter {
    pub quote: String,
    pub chapter_number: i32,
}

pub struct AnchorQuoteIndex {
    chapter_numbers: HashMap<i64, i32>,
    anchors_by_id: HashMap<i64, (Option<i64>, String)>,
}

impl AnchorQuoteIndex {
    pub fn new<C: SourceChapter, A: SourceTextAnchor>(chapters: &[C], text_anchors: &[A]) -> Self {
        let chapter_numbers = chapters
            .iter()
            .map(|chapter| (chapter.source_chapter_id(), chapter.source_chapter_number()))
            .collect();
        let mut anchors_by_id = HashMap::with_capacity(text_anchors.len());
        for anchor in text_anchors {
            anchors_by_id
                .entry(anchor.source_anchor_id())
                .or_insert_with(|| {
                    (
                        anchor.source_anchor_chapter_id(),
                        anchor.source_anchor_quote().to_string(),
                    )
                });
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
        let (source_chapter_id, quote) = self.anchors_by_id.get(&anchor_id).ok_or_else(|| {
            format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}")
        })?;
        let source_chapter_id = source_chapter_id
            .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不是章节正文锚点"))?;
        let chapter_number = self
            .chapter_numbers
            .get(&source_chapter_id)
            .copied()
            .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不属于本次输入章节"))?;
        Ok(AnchorQuoteWithChapter {
            quote: quote.clone(),
            chapter_number,
        })
    }
}

pub fn anchor_quote_with_chapter_number<C: SourceChapter, A: SourceTextAnchor>(
    path: &str,
    anchor_id: i64,
    chapters: &[C],
    text_anchors: &[A],
) -> Result<AnchorQuoteWithChapter, String> {
    AnchorQuoteIndex::new(chapters, text_anchors).quote_with_chapter_number(path, anchor_id)
}

pub fn anchor_quotes_with_chapter_numbers<C: SourceChapter, A: SourceTextAnchor>(
    path: &str,
    anchor_ids: &[i64],
    chapters: &[C],
    text_anchors: &[A],
) -> Result<Vec<AnchorQuoteWithChapter>, String> {
    let index = AnchorQuoteIndex::new(chapters, text_anchors);
    anchor_ids
        .iter()
        .map(|anchor_id| index.quote_with_chapter_number(path, *anchor_id))
        .collect()
}

pub fn anchor_quotes_in_exact_chapter<A: SourceTextAnchor>(
    path: &str,
    anchor_ids: &[i64],
    text_anchors: &[A],
    chapter_id: i64,
    scope_label: &str,
) -> Result<Vec<String>, String> {
    anchor_ids
        .iter()
        .map(|anchor_id| {
            let anchor = text_anchors
                .iter()
                .find(|anchor| anchor.source_anchor_id() == *anchor_id)
                .ok_or_else(|| {
                    format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}")
                })?;
            if anchor.source_anchor_chapter_id() != Some(chapter_id) {
                return Err(format!(
                    "{path} 无效：text_anchor_id={anchor_id} 不属于{scope_label}"
                ));
            }
            Ok(anchor.source_anchor_quote().to_string())
        })
        .collect()
}

pub fn text_anchor_prompt_values<C: SourceChapter, A: SourceTextAnchor>(
    chapters: &[C],
    anchors: &[A],
) -> Vec<Value> {
    let chapter_numbers = chapters
        .iter()
        .map(|chapter| (chapter.source_chapter_id(), chapter.source_chapter_number()))
        .collect::<HashMap<_, _>>();

    anchors
        .iter()
        .map(|anchor| {
            let chapter_number = anchor
                .source_anchor_chapter_id()
                .and_then(|chapter_id| chapter_numbers.get(&chapter_id).copied());
            json!({
                "id": anchor.source_anchor_id(),
                "chapter_label": chapter_number.map(|value| format!("第{value}章")),
                "quote": anchor.source_anchor_quote(),
            })
        })
        .collect()
}
