use super::sample_chapter;
use super::*;
use a4n_domain::source_spans::SourceSpan;

fn source_span_records(spans: &[SourceSpan]) -> Vec<Value> {
    spans
        .iter()
        .map(|span| {
            json!({
                "id": span.id,
                "paragraph_index": span.paragraph_index,
                "text": span.text,
            })
        })
        .collect()
}

#[path = "materialize/rejections.rs"]
mod rejections;
#[path = "materialize/success.rs"]
mod success;
