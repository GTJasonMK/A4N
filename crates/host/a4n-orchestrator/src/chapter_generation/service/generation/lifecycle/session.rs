use std::sync::Arc;

use a4n_db::chapter_content::ChapterGenerationExistingRecord;
use a4n_domain::chapter::chapter_status::STATUS_GENERATING;
use a4n_error::AppError;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use super::super::super::ChapterGenerationService;
mod transition;

pub(in crate::chapter_generation) struct ChapterGenerationSession {
    pub(super) project_id: i64,
    pub(super) chapter_number: i32,
    pub(super) existing_chapter: Option<ExistingChapterGeneration>,
    _guard: OwnedMutexGuard<()>,
}

pub(super) struct ExistingChapterGeneration {
    pub(super) id: i64,
    pub(super) previous_status: String,
    pub(super) title: Option<String>,
    pub(super) summary: Option<String>,
}

impl From<ChapterGenerationExistingRecord> for ExistingChapterGeneration {
    fn from(record: ChapterGenerationExistingRecord) -> Self {
        Self {
            id: record.id,
            previous_status: record.previous_status,
            title: record.title,
            summary: record.summary,
        }
    }
}

impl ChapterGenerationService {
    fn try_generation_guard(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<OwnedMutexGuard<()>, AppError> {
        let key = (project_id, chapter_number);
        let lock = self
            .generation_locks
            .entry(key)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        lock.try_lock_owned().map_err(|_| AppError::InvalidState {
            current: STATUS_GENERATING.to_string(),
            operation: format!("生成第{chapter_number}章"),
        })
    }
}
