//! Source text evidence values and pure projection algorithms.
//!
//! Database rows implement the small input traits in their owning adapter
//! crate. The algorithms here never depend on ORM entities or persistence.

use serde_json::{Value, json};
use std::collections::HashMap;

/// Minimal chapter input required to derive source spans.
pub trait SourceChapter {
    fn source_chapter_id(&self) -> i64;
    fn source_chapter_number(&self) -> i32;
    fn source_content(&self) -> &str;
}

/// Minimal text-anchor input required by prompt and evidence projections.
pub trait SourceTextAnchor {
    fn source_anchor_id(&self) -> i64;
    fn source_anchor_chapter_id(&self) -> Option<i64>;
    fn source_anchor_quote(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub id: String,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub paragraph_index: usize,
    pub sentence_index: usize,
    pub fragment_index: usize,
    pub text: String,
    pub char_start: usize,
    pub char_end: usize,
}

pub struct SourceSpanIndex<'a> {
    spans_by_id: HashMap<&'a str, &'a SourceSpan>,
}

impl<'a> SourceSpanIndex<'a> {
    pub fn new(source_spans: &'a [SourceSpan]) -> Self {
        Self {
            spans_by_id: source_spans.iter().fold(
                HashMap::with_capacity(source_spans.len()),
                |mut spans_by_id, span| {
                    spans_by_id.entry(span.id.as_str()).or_insert(span);
                    spans_by_id
                },
            ),
        }
    }

    pub fn resolve_text(&self, id: &str) -> Result<String, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("source_span_id 不能为空".to_string());
        }
        self.spans_by_id
            .get(id)
            .map(|span| span.text.clone())
            .ok_or_else(|| format!("source_span_id 不在本次输入候选中: {id}"))
    }
}

pub fn source_spans_for_chapters<C: SourceChapter>(chapters: &[C]) -> Vec<SourceSpan> {
    chapters.iter().flat_map(source_spans_for_chapter).collect()
}

pub fn source_spans_for_content(
    chapter_id: i64,
    chapter_number: i32,
    content: &str,
) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    for (sentence_index, (sentence_start, sentence_end)) in
        split_ranges(content, is_sentence_delimiter)
            .into_iter()
            .enumerate()
    {
        let sentence = &content[sentence_start..sentence_end];
        let mut fragment_index = 0usize;
        for (relative_start, relative_end) in split_ranges(sentence, is_fragment_delimiter) {
            let absolute_start = sentence_start + relative_start;
            let absolute_end = sentence_start + relative_end;
            let Some((start, end)) = trim_content_range(content, absolute_start, absolute_end)
            else {
                continue;
            };
            fragment_index += 1;
            spans.push(SourceSpan {
                id: format!(
                    "ch{}:s{}:f{}",
                    chapter_number,
                    sentence_index + 1,
                    fragment_index
                ),
                chapter_id,
                chapter_number,
                paragraph_index: paragraph_index_for_byte(content, start),
                sentence_index: sentence_index + 1,
                fragment_index,
                text: content[start..end].to_string(),
                char_start: content[..start].chars().count(),
                char_end: content[..end].chars().count(),
            });
        }
    }
    spans
}

pub fn source_span_prompt_values(spans: &[SourceSpan]) -> Vec<Value> {
    spans.iter().map(source_span_prompt_value).collect()
}

pub fn source_span_prompt_value(span: &SourceSpan) -> Value {
    json!({
        "id": span.id,
        "paragraph_index": span.paragraph_index,
        "text": span.text,
    })
}

pub fn resolve_source_span_text(spans: &[SourceSpan], id: &str) -> Result<String, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("source_span_id 不能为空".to_string());
    }
    spans
        .iter()
        .find(|span| span.id == id)
        .map(|span| span.text.clone())
        .ok_or_else(|| format!("source_span_id 不在本次输入候选中: {id}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorQuoteWithChapter {
    pub quote: String,
    pub chapter_number: i32,
}

/// An owned lookup index keeps the pure contract independent of row lifetimes.
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

/// Projects persisted text anchors into prompt evidence fields only.
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

#[derive(Debug, Clone, Copy)]
pub struct ChapterSourceContext<'a> {
    pub content: &'a str,
}

pub fn source_metadata(source: ChapterSourceContext<'_>) -> Value {
    json!({
        "content_sha256": crate::source_text_hash(source.content),
        "content_chars": source.content.chars().count(),
    })
}

pub fn source_metadata_with_flags(
    source: ChapterSourceContext<'_>,
    flags: impl IntoIterator<Item = (&'static str, bool)>,
) -> Value {
    let mut metadata = source_metadata(source);
    if let Value::Object(map) = &mut metadata {
        for (key, value) in flags {
            map.insert(key.into(), json!(value));
        }
    }
    metadata
}

pub fn metadata_source_content_sha256(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|value| value.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn source_spans_for_chapter<C: SourceChapter>(chapter: &C) -> Vec<SourceSpan> {
    source_spans_for_content(
        chapter.source_chapter_id(),
        chapter.source_chapter_number(),
        chapter.source_content(),
    )
}

fn split_ranges(content: &str, is_delimiter: fn(char) -> bool) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for (index, ch) in content.char_indices() {
        if is_delimiter(ch) {
            ranges.push((start, index));
            start = index + ch.len_utf8();
        }
    }
    ranges.push((start, content.len()));
    ranges
}

fn trim_content_range(content: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let text = &content[start..end];
    let trimmed = text.trim_matches(|ch: char| ch.is_whitespace() || is_outer_punctuation(ch));
    if trimmed.is_empty() {
        return None;
    }
    let trimmed_start = text.find(trimmed)?;
    Some((start + trimmed_start, start + trimmed_start + trimmed.len()))
}

fn paragraph_index_for_byte(content: &str, byte_index: usize) -> usize {
    let mut paragraph_index = 1usize;
    let mut in_line_break = false;
    for (index, ch) in content.char_indices() {
        if index >= byte_index {
            break;
        }
        if matches!(ch, '\n' | '\r') {
            if !in_line_break {
                paragraph_index += 1;
                in_line_break = true;
            }
        } else if !ch.is_whitespace() {
            in_line_break = false;
        }
    }
    paragraph_index
}

fn is_sentence_delimiter(ch: char) -> bool {
    matches!(ch, '。' | '！' | '？' | '!' | '?' | '\n' | '\r')
}

fn is_fragment_delimiter(ch: char) -> bool {
    matches!(ch, '，' | '、' | '；' | '：' | ',' | ';' | ':')
}

fn is_outer_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '，' | '、'
            | '；'
            | '：'
            | ','
            | ';'
            | ':'
            | '。'
            | '！'
            | '？'
            | '!'
            | '?'
            | '"'
            | '\''
            | '“'
            | '”'
            | '‘'
            | '’'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Chapter {
        id: i64,
        number: i32,
        content: String,
    }

    impl SourceChapter for Chapter {
        fn source_chapter_id(&self) -> i64 {
            self.id
        }

        fn source_chapter_number(&self) -> i32 {
            self.number
        }

        fn source_content(&self) -> &str {
            &self.content
        }
    }

    struct Anchor {
        id: i64,
        chapter_id: Option<i64>,
        quote: String,
    }

    impl SourceTextAnchor for Anchor {
        fn source_anchor_id(&self) -> i64 {
            self.id
        }

        fn source_anchor_chapter_id(&self) -> Option<i64> {
            self.chapter_id
        }

        fn source_anchor_quote(&self) -> &str {
            &self.quote
        }
    }

    #[test]
    fn derives_spans_from_pure_chapter_input() {
        let chapter = Chapter {
            id: 7,
            number: 1,
            content: "他转身走向厨房，地上有一把军用匕首，刀身上还沾着血迹。".to_string(),
        };
        let spans = source_spans_for_chapters(&[chapter]);
        assert!(spans.iter().any(|span| span.text == "地上有一把军用匕首"));
        assert!(spans.iter().all(|span| span.char_start < span.char_end));
    }

    #[test]
    fn prompt_projection_keeps_only_stable_fields() {
        let chapter = Chapter {
            id: 10,
            number: 3,
            content: "正文".to_string(),
        };
        let anchor = Anchor {
            id: 20,
            chapter_id: Some(10),
            quote: "潮汐城".to_string(),
        };
        let value = text_anchor_prompt_values(&[chapter], &[anchor]);
        assert_eq!(
            value[0],
            json!({"id": 20, "chapter_label": "第3章", "quote": "潮汐城"})
        );
    }

    #[test]
    fn anchor_index_reports_scope_errors() {
        let chapters = [Chapter {
            id: 10,
            number: 3,
            content: "正文".to_string(),
        }];
        let anchors = [Anchor {
            id: 20,
            chapter_id: Some(10),
            quote: "潮汐城".to_string(),
        }];
        assert_eq!(
            anchor_quote_with_chapter_number("evidence", 20, &chapters, &anchors)
                .expect("valid anchor"),
            AnchorQuoteWithChapter {
                quote: "潮汐城".to_string(),
                chapter_number: 3,
            }
        );
        assert_eq!(
            anchor_quotes_in_exact_chapter("evidence", &[20], &anchors, 11, "当前章节"),
            Err("evidence 无效：text_anchor_id=20 不属于当前章节".to_string())
        );
    }

    #[test]
    fn metadata_and_span_index_are_pure() {
        let metadata = source_metadata(ChapterSourceContext { content: "正文" });
        assert_eq!(metadata["content_chars"], 2);
        let spans = source_spans_for_content(1, 1, "绷带。");
        assert_eq!(
            SourceSpanIndex::new(&spans).resolve_text(&spans[0].id),
            Ok("绷带".to_string())
        );
    }

    #[test]
    fn nested_hash_extraction_requires_non_empty_hashes() {
        let nested = json!({ "source": { "content_sha256": "same" } });
        assert_eq!(metadata_source_content_sha256(Some(&nested)), Some("same"));
        assert_eq!(metadata_source_content_sha256(Some(&json!({}))), None);
    }
}
