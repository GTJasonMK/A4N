//! Adapters from persistence rows to pure domain input contracts.

use a4n_domain::{
    chapter::ChapterAnalysisSource,
    source_spans::{SourceChapter, SourceTextAnchor},
};

use crate::models::{chapter::chapter, project::text_anchor};

impl SourceChapter for chapter::Model {
    fn source_chapter_id(&self) -> i64 {
        self.id
    }

    fn source_chapter_number(&self) -> i32 {
        self.chapter_number
    }

    fn source_content(&self) -> &str {
        &self.content
    }
}

impl SourceTextAnchor for text_anchor::Model {
    fn source_anchor_id(&self) -> i64 {
        self.id
    }

    fn source_anchor_chapter_id(&self) -> Option<i64> {
        self.source_chapter_id
    }

    fn source_anchor_quote(&self) -> &str {
        &self.quote
    }
}

impl ChapterAnalysisSource for chapter::Model {
    fn has_available_analysis_content(&self) -> bool {
        self.status != chapter::STATUS_DELETED && !self.content.trim().is_empty()
    }
}
