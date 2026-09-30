use super::types::{SourceChapter, SourceSpan, SourceTextAnchor};
use serde_json::{Value, json};

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

pub fn source_span_prompt_excerpt(content: &str, spans: &[SourceSpan]) -> Result<String, String> {
    source_span_excerpt(content, spans, true)
}

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
