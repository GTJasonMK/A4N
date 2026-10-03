use super::types::{SourceChapter, SourceSpan, SourceTextAnchor};
use std::collections::HashMap;

pub fn source_span_marker_text(
    content: &str,
    spans: &[SourceSpan],
    markers_by_id: &HashMap<String, String>,
) -> Result<String, String> {
    source_marker_text(
        content,
        spans.iter().map(|span| {
            (
                span.id.clone(),
                span.char_start,
                span.char_end,
                span.text.clone(),
            )
        }),
        markers_by_id,
    )
}

pub fn source_marker_text(
    content: &str,
    ranges: impl IntoIterator<Item = (String, usize, usize, String)>,
    markers_by_id: &HashMap<String, String>,
) -> Result<String, String> {
    let ranges = ranges.into_iter().collect::<Vec<_>>();
    let validated = validated_source_ranges(
        content,
        ranges
            .iter()
            .map(|(id, start, end, quote)| (id.clone(), *start, *end, quote.as_str()))
            .collect(),
    )?;
    mark_source_ranges(content, &validated, markers_by_id, 0, content.len())
}

pub fn source_span_context_excerpt(content: &str, spans: &[SourceSpan]) -> Result<String, String> {
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
    let excerpts = source_excerpt_windows(content, &ranges)
        .into_iter()
        .map(|(start, end)| content[start..end].to_string())
        .collect::<Vec<_>>();
    Ok(excerpts.join("\n\n[非当前任务段落已省略]\n\n"))
}

pub fn source_span_marker_excerpt(
    content: &str,
    spans: &[SourceSpan],
    markers_by_id: &HashMap<String, String>,
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
    source_excerpt_windows(content, &ranges)
        .into_iter()
        .map(|(start, end)| mark_source_ranges(content, &ranges, markers_by_id, start, end))
        .collect::<Result<Vec<_>, _>>()
        .map(|excerpts| excerpts.join("\n\n[非当前任务段落已省略]\n\n"))
}

fn source_excerpt_windows(content: &str, ranges: &[(String, usize, usize)]) -> Vec<(usize, usize)> {
    let mut windows = Vec::new();
    for (_, start, end) in ranges {
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
    merged
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

fn mark_source_ranges(
    content: &str,
    ranges: &[(String, usize, usize)],
    markers_by_id: &HashMap<String, String>,
    start: usize,
    end: usize,
) -> Result<String, String> {
    let mut result = String::with_capacity(end - start);
    let mut cursor = start;
    for (id, range_start, range_end) in ranges {
        if *range_start < start || *range_end > end {
            continue;
        }
        if *range_start < cursor {
            return Err("来源标记位置重叠".to_string());
        }
        let marker = markers_by_id
            .get(id)
            .ok_or_else(|| format!("原文引用 {id} 缺少本次请求的短标记"))?;
        result.push_str(&content[cursor..*range_start]);
        result.push_str(marker);
        result.push(' ');
        result.push_str(&content[*range_start..*range_end]);
        cursor = *range_end;
    }
    result.push_str(&content[cursor..end]);
    Ok(result)
}

#[cfg(test)]
mod marker_tests {
    use super::*;

    #[test]
    fn source_markers_preserve_fragment_boundaries_and_paragraphs() {
        let content = "甲，乙。\n丙。";
        let spans = super::super::spans::source_spans_for_content(7, 1, content);
        let markers = HashMap::from([
            (spans[0].id.clone(), "#0".to_string()),
            (spans[1].id.clone(), "#1".to_string()),
            (spans[2].id.clone(), "#2".to_string()),
        ]);

        let marked = source_span_marker_text(content, &spans, &markers).unwrap();

        assert_eq!(marked, "#0 甲，#1 乙。\n#2 丙。");
    }

    #[test]
    fn source_markers_preserve_reserved_symbols_in_original_text() {
        let content = "原文¶符号#x。";
        let spans = super::super::spans::source_spans_for_content(7, 1, content);
        let markers = HashMap::from([(spans[0].id.clone(), "#0".to_string())]);

        let marked = source_span_marker_text(content, &spans, &markers).unwrap();

        assert_eq!(marked, "#0 原文¶符号#x。");
    }

    #[test]
    fn source_markers_are_separated_from_ascii_source_text() {
        let content = "Alice enters.\nShe opens the door.";
        let spans = super::super::spans::source_spans_for_content(7, 1, content);
        let markers = HashMap::from([
            (spans[0].id.clone(), "#0".to_string()),
            (spans[1].id.clone(), "#1".to_string()),
        ]);

        let marked = source_span_marker_text(content, &spans, &markers).unwrap();

        assert_eq!(marked, "#0 Alice enters.\n#1 She opens the door.");
    }

    #[test]
    fn source_markers_reject_stale_quotes_and_ranges() {
        let content = "可信原文。";
        let mut spans = super::super::spans::source_spans_for_content(7, 1, content);
        let markers = HashMap::from([(spans[0].id.clone(), "#0".to_string())]);

        spans[0].text = "过期引文".to_string();
        assert!(source_span_marker_text(content, &spans, &markers).is_err());

        spans[0].text = "可信原文".to_string();
        spans[0].char_end = usize::MAX;
        assert!(source_span_marker_text(content, &spans, &markers).is_err());
    }

    #[test]
    fn marker_excerpt_keeps_selected_paragraphs_and_marks_omissions() {
        let content = "甲句。\n\n乙句。\n\n丙句。";
        let spans = super::super::spans::source_spans_for_content(7, 1, content);
        let markers = HashMap::from([
            (spans[0].id.clone(), "#0".to_string()),
            (spans[1].id.clone(), "#1".to_string()),
            (spans[2].id.clone(), "#2".to_string()),
        ]);

        let excerpt =
            source_span_marker_excerpt(content, &[spans[0].clone(), spans[2].clone()], &markers)
                .unwrap();

        assert!(excerpt.contains("#0 甲句。"));
        assert!(excerpt.contains("[非当前任务段落已省略]"));
        assert!(excerpt.contains("#2 丙句。"));
        assert!(!excerpt.contains("乙句"));
    }
}
