use a4n_core::source_spans::{SourceSpan, source_span_prompt_values};
use a4n_db::project::ResolvedChapterSource;
use serde_json::{Value, json};

pub(super) fn shooting_script_user_payload(
    source: &ResolvedChapterSource,
    source_spans: &[SourceSpan],
) -> Value {
    json!({
        "source": {
            "root_project_id": source.root_project_id,
            "source_chapter_id": source.source_chapter_id,
            "chapter_number": source.chapter_number,
            "title": source.title,
            "summary": source.summary,
            "analysis": source.analysis_data_json,
            "content": source.content,
            "content_sha256": source.content_sha256,
            "source_revision_key": source.source_revision_key,
        },
        "source_spans": source_span_prompt_values(source_spans),
    })
}
