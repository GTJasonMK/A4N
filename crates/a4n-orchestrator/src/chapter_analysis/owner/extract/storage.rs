use a4n_core::{chapter_analysis::summary_text, models::chapter::chapter};
use serde_json::{Value, json};

use super::{merge_tags, metadata_tags, should_replace_summary};

#[derive(Debug, Clone, PartialEq)]
pub struct ChapterAnalysisStorageProjection {
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
}

pub fn chapter_analysis_storage_projection(
    chapter: &chapter::Model,
    analysis: &Value,
) -> ChapterAnalysisStorageProjection {
    let summary = should_replace_summary(chapter.summary.as_deref(), chapter.chapter_number)
        .then(|| summary_text(analysis))
        .flatten();
    let tags = merge_tags(chapter.style_tags.as_ref(), metadata_tags(analysis));
    ChapterAnalysisStorageProjection {
        summary,
        style_tags: (!tags.is_empty()).then(|| json!(tags)),
    }
}
