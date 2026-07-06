use a4n_core::models::chapter::chapter;
use serde_json::{Value, json};

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

pub fn source_spans_for_chapters(chapters: &[chapter::Model]) -> Vec<SourceSpan> {
    chapters.iter().flat_map(source_spans_for_chapter).collect()
}

pub(crate) fn source_spans_for_content(
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

pub(crate) fn source_span_prompt_values(spans: &[SourceSpan]) -> Vec<Value> {
    spans.iter().map(source_span_prompt_value).collect()
}

pub(crate) fn source_span_prompt_value(span: &SourceSpan) -> Value {
    json!({
        "id": span.id,
        "paragraph_index": span.paragraph_index,
        "text": span.text,
    })
}

pub(crate) fn resolve_source_span_text(spans: &[SourceSpan], id: &str) -> Result<String, String> {
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

fn source_spans_for_chapter(chapter: &chapter::Model) -> Vec<SourceSpan> {
    source_spans_for_content(chapter.id, chapter.chapter_number, &chapter.content)
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

    fn chapter_for_source_span_test(content: &str) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: 7,
            project_id: 1,
            outline_id: None,
            chapter_number: 1,
            title: Some("测试".to_string()),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn source_spans_are_contiguous_original_substrings() {
        let chapter = chapter_for_source_span_test(
            "他转身走向厨房，地上有一把军用匕首，刀身上还沾着前一个住户的血迹。",
        );

        let spans = source_spans_for_chapters(std::slice::from_ref(&chapter));
        let texts = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>();

        assert!(texts.contains(&"他转身走向厨房"));
        assert!(texts.contains(&"地上有一把军用匕首"));
        assert!(texts.contains(&"刀身上还沾着前一个住户的血迹"));
        for span in spans {
            let start = chapter
                .content
                .char_indices()
                .nth(span.char_start)
                .map(|(index, _)| index)
                .unwrap_or(chapter.content.len());
            let end = chapter
                .content
                .char_indices()
                .nth(span.char_end)
                .map(|(index, _)| index)
                .unwrap_or(chapter.content.len());
            assert_eq!(chapter.content[start..end], span.text);
        }
    }

    #[test]
    fn source_spans_keep_short_object_names() {
        let chapter = chapter_for_source_span_test("绷带。匕首。");
        let spans = source_spans_for_chapters(&[chapter]);
        let texts = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>();

        assert!(texts.contains(&"绷带"));
        assert!(texts.contains(&"匕首"));
    }

    #[test]
    fn source_span_prompt_values_expose_id_paragraph_and_text() {
        let chapter = chapter_for_source_span_test("林岚走进旧城，墙上刻着隐藏标记。");
        let spans = source_spans_for_chapters(&[chapter]);
        let values = source_span_prompt_values(&spans);
        let first = values
            .first()
            .and_then(Value::as_object)
            .expect("prompt span is an object");

        assert_eq!(
            first.keys().cloned().collect::<Vec<_>>(),
            vec![
                "id".to_string(),
                "paragraph_index".to_string(),
                "text".to_string(),
            ]
        );
        assert!(first.get("id").is_some());
        assert!(first.get("paragraph_index").is_some());
        assert!(first.get("text").is_some());
        assert!(first.get("chapter_id").is_none());
        assert!(first.get("chapter_number").is_none());
        assert!(first.get("char_start").is_none());
        assert!(first.get("char_end").is_none());
        assert!(first.get("long_text").is_none());
    }

    #[test]
    fn source_spans_track_paragraph_index() {
        let chapter =
            chapter_for_source_span_test("第一段第一句，第一段第二节。\n\n第二段第一句。");
        let spans = source_spans_for_chapters(&[chapter]);

        assert_eq!(spans[0].paragraph_index, 1);
        assert_eq!(spans[1].paragraph_index, 1);
        assert_eq!(spans[2].paragraph_index, 2);
    }

    #[test]
    fn resolve_source_span_rejects_unknown_id() {
        let chapter = chapter_for_source_span_test("绷带。");
        let spans = source_spans_for_chapters(&[chapter]);

        let err = resolve_source_span_text(&spans, "ch1:s9:f9").expect_err("unknown id fails");

        assert!(err.contains("source_span_id 不在本次输入候选中"));
    }
}
