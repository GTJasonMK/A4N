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
    fn source_anchor_char_range(&self) -> (i32, i32);
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

/// 在原文上标注引用 ID，保留引号、标点、空白和段落，避免另发一份分句正文。
pub fn source_span_prompt_text(content: &str, spans: &[SourceSpan]) -> Result<String, String> {
    annotate_source_ranges(
        content,
        spans
            .iter()
            .map(|span| {
                (
                    span.id.clone(),
                    span.char_start,
                    span.char_end,
                    span.text.as_str(),
                )
            })
            .collect(),
    )
}

/// 仅提供被选中引用所在的完整段落；段落内仍保留原标点、引号和引用身份。
pub fn source_span_prompt_excerpt(content: &str, spans: &[SourceSpan]) -> Result<String, String> {
    source_span_excerpt(content, spans, true)
}

/// 不需要模型选择引用 ID 的任务只接收所选段落原文。
pub fn source_span_context_excerpt(content: &str, spans: &[SourceSpan]) -> Result<String, String> {
    source_span_excerpt(content, spans, false)
}

fn source_span_excerpt(
    content: &str,
    spans: &[SourceSpan],
    annotate: bool,
) -> Result<String, String> {
    let ranges = validated_source_ranges(
        content,
        spans
            .iter()
            .map(|span| {
                (
                    span.id.clone(),
                    span.char_start,
                    span.char_end,
                    span.text.as_str(),
                )
            })
            .collect(),
    )?;
    let mut windows = Vec::new();
    for (_, start, end) in &ranges {
        let start = content[..*start]
            .rfind(['\n', '\r'])
            .map_or(0, |index| index + 1);
        let end = content[*end..]
            .find(['\n', '\r'])
            .map_or(content.len(), |index| end + index);
        windows.push((start, end));
    }
    windows.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in windows {
        if let Some(last) = merged.last_mut()
            && (start <= last.1 || content[last.1..start].trim().is_empty())
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    let excerpts = merged
        .into_iter()
        .map(|(start, end)| {
            if !annotate {
                return content[start..end].to_string();
            }
            annotate_byte_ranges(content, &ranges, start, end)
        })
        .collect::<Vec<_>>();
    Ok(excerpts.join("\n\n[非当前任务段落已省略]\n\n"))
}

pub fn text_anchor_prompt_text<C: SourceChapter, A: SourceTextAnchor>(
    chapter: &C,
    anchors: &[A],
) -> Result<String, String> {
    let ranges = anchors
        .iter()
        .filter(|anchor| anchor.source_anchor_chapter_id() == Some(chapter.source_chapter_id()))
        .map(|anchor| {
            let id = anchor.source_anchor_id().to_string();
            let (start, end) = anchor.source_anchor_char_range();
            let start = usize::try_from(start).map_err(|_| format!("原文锚点 {id} 起点无效"))?;
            let end = usize::try_from(end).map_err(|_| format!("原文锚点 {id} 终点无效"))?;
            Ok((id, start, end, anchor.source_anchor_quote()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    annotate_source_ranges(chapter.source_content(), ranges)
}

fn annotate_source_ranges(
    content: &str,
    ranges: Vec<(String, usize, usize, &str)>,
) -> Result<String, String> {
    let ranges = validated_source_ranges(content, ranges)?;
    Ok(annotate_byte_ranges(content, &ranges, 0, content.len()))
}

fn validated_source_ranges(
    content: &str,
    mut ranges: Vec<(String, usize, usize, &str)>,
) -> Result<Vec<(String, usize, usize)>, String> {
    let offsets = content
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(content.len()))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|(_, start, end, _)| (*start, *end));
    let mut validated = Vec::with_capacity(ranges.len());
    let mut cursor = 0;
    for (id, start, end, quote) in ranges {
        let bounds = offsets.get(start).zip(offsets.get(end));
        let Some((&start, &end)) = bounds.filter(|(start, end)| start < end) else {
            return Err(format!("原文引用 {id} 的字符范围无效"));
        };
        if start < cursor || &content[start..end] != quote {
            return Err(format!("原文引用 {id} 已失效或与其他引用重叠"));
        }
        validated.push((id, start, end));
        cursor = end;
    }
    Ok(validated)
}

fn annotate_byte_ranges(
    content: &str,
    ranges: &[(String, usize, usize)],
    start: usize,
    end: usize,
) -> String {
    let mut result = String::with_capacity(end - start);
    let mut cursor = start;
    for (id, range_start, range_end) in ranges {
        if *range_start < start || *range_end > end {
            continue;
        }
        result.push_str(&content[cursor..*range_start]);
        result.push_str(&format!("⟦{id}⟧"));
        result.push_str(&content[*range_start..*range_end]);
        cursor = *range_end;
    }
    result.push_str(&content[cursor..end]);
    result
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

        fn source_anchor_char_range(&self) -> (i32, i32) {
            (0, self.quote.chars().count() as i32)
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
    fn annotated_source_preserves_original_text_and_rejects_stale_ranges() {
        let content = "“别走！”她说。\n\n雨落下来，🌧️打湿窗台。";
        let mut spans = source_spans_for_content(7, 3, content);
        let annotated = source_span_prompt_text(content, &spans).expect("valid source");
        let restored = spans.iter().fold(annotated.clone(), |text, span| {
            text.replace(&format!("⟦{}⟧", span.id), "")
        });
        assert_eq!(restored, content);
        assert!(annotated.contains("！”"));
        assert!(annotated.contains("\n\n"));
        assert!(
            annotated.len()
                < serde_json::to_string(&source_span_prompt_values(&spans))
                    .unwrap()
                    .len()
        );
        spans[0].text = "已变更".to_string();
        assert!(source_span_prompt_text(content, &spans).is_err());
        spans[0].char_end = usize::MAX;
        assert!(source_span_prompt_text(content, &spans).is_err());
    }

    #[test]
    fn scoped_excerpt_keeps_quoted_paragraphs_without_unrelated_story() {
        let text = "“别走！”她说。\n\n不相关的往事。\n\n🌧️雨落下来，门关上了。";
        let spans = source_spans_for_content(7, 2, text);
        let selected = spans
            .iter()
            .filter(|span| span.text == "别走" || span.text.contains("雨落下来"))
            .cloned()
            .collect::<Vec<_>>();
        let excerpt = source_span_prompt_excerpt(text, &selected).unwrap();
        assert!(excerpt.contains("！”她说。"));
        assert!(excerpt.contains("门关上了。"));
        assert!(!excerpt.contains("不相关的往事"));
        let plain = source_span_context_excerpt(text, &selected).unwrap();
        assert!(plain.contains("“别走！”她说。"));
        assert!(!plain.contains("⟦"));
        assert!(!plain.contains("不相关的往事"));
        for span in &selected {
            assert_eq!(excerpt.matches(&format!("⟦{}⟧", span.id)).count(), 1);
        }
        assert_eq!(source_span_prompt_excerpt(text, &[]).unwrap(), "");
    }

    #[test]
    fn annotated_anchor_content_uses_only_current_chapter_and_valid_quotes() {
        let chapter = Chapter {
            id: 10,
            number: 3,
            content: "潮汐城。\n".into(),
        };
        let mut anchors = vec![
            Anchor {
                id: 20,
                chapter_id: Some(10),
                quote: "潮汐城".into(),
            },
            Anchor {
                id: 21,
                chapter_id: Some(11),
                quote: "其他章节".into(),
            },
        ];
        assert_eq!(
            text_anchor_prompt_text(&chapter, &anchors).unwrap(),
            "⟦20⟧潮汐城。\n"
        );
        anchors[0].quote = "旧正文".into();
        assert!(text_anchor_prompt_text(&chapter, &anchors).is_err());
    }

    #[test]
    fn excerpts_share_range_validation_and_respect_all_line_endings() {
        for newline in ["\n", "\r", "\r\n"] {
            let text = format!("前文。{newline}“别走！”她说。{newline}后文。");
            let spans = source_spans_for_content(7, 2, &text);
            let span = spans.iter().find(|span| span.text == "别走").unwrap();
            let selected = std::slice::from_ref(span);
            assert_eq!(
                source_span_context_excerpt(&text, selected).unwrap(),
                "“别走！”她说。"
            );
            assert_eq!(
                source_span_prompt_excerpt(&text, selected).unwrap(),
                format!("“⟦{}⟧别走！”她说。", span.id),
            );
            let overlapping = [span.clone(), span.clone()];
            assert!(source_span_context_excerpt(&text, &overlapping).is_err());
            assert!(source_span_prompt_excerpt(&text, &overlapping).is_err());
        }
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
