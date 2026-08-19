mod anchor_contract;
mod anchor_prompt;
mod metadata;
mod spans;

pub use anchor_contract::{
    AnchorQuoteWithChapter, anchor_quote_with_chapter_number, anchor_quotes_in_exact_chapter,
    anchor_quotes_with_chapter_numbers,
};
pub use anchor_prompt::text_anchor_prompt_values;
pub use metadata::{
    ChapterSourceContext, metadata_source_content_sha256, source_metadata,
    source_metadata_with_flags,
};
pub use spans::{
    SourceSpan, resolve_source_span_text, source_span_prompt_value, source_span_prompt_values,
    source_spans_for_chapters, source_spans_for_content, source_text_hash,
};

#[cfg(test)]
mod tests;
