use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::models::chapter::chapter;

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

pub fn source_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
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
