use super::types::{SourceChapter, SourceSpan};

pub fn source_spans_for_chapters<C: SourceChapter>(chapters: &[C]) -> Vec<SourceSpan> {
    chapters.iter().flat_map(source_spans_for_chapter).collect()
}

pub fn source_paragraph_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut offset = 0;
    content
        .split_inclusive(['\n', '\r'])
        .filter_map(|line| {
            let start = offset;
            offset += line.len();
            let paragraph = line.trim_end();
            (!paragraph.trim().is_empty()).then_some((start, start + paragraph.len()))
        })
        .collect()
}

pub fn source_spans_for_content(
    chapter_id: i64,
    chapter_number: i32,
    content: &str,
) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    let paragraphs = source_paragraph_ranges(content);
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
                paragraph_index: paragraphs.partition_point(|(_, end)| *end <= start) + 1,
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
