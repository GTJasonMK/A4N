use std::collections::HashMap;

pub trait SourceChapter {
    fn source_chapter_id(&self) -> i64;
    fn source_chapter_number(&self) -> i32;
    fn source_content(&self) -> &str;
}

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
