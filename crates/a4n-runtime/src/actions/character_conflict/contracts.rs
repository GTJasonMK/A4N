use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub type CharacterConflictProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Copy)]
pub struct CharacterConflictTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub(super) struct CharacterConflictRequestContext {
    pub request: ChatRequest,
    pub source_ids: BTreeSet<String>,
    pub call_key: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharacterConflictChapterScope {
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

impl CharacterConflictChapterScope {
    pub fn new(start_chapter: Option<i32>, end_chapter: Option<i32>) -> Result<Self, AppError> {
        if start_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节必须大于 0".to_string(),
            ));
        }
        if end_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查结束章节必须大于 0".to_string(),
            ));
        }
        if let (Some(start), Some(end)) = (start_chapter, end_chapter)
            && start > end
        {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节不能大于结束章节".to_string(),
            ));
        }
        Ok(Self {
            start_chapter,
            end_chapter,
        })
    }

    pub(super) fn to_context_value(self) -> Value {
        json!({
            "start_chapter": self.start_chapter,
            "end_chapter": self.end_chapter,
        })
    }
}
