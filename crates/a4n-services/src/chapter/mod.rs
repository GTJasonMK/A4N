//! 章节生成服务：核心写作引擎
//!
//! 对应文档 05-章节生成系统

pub mod analysis;
mod content;
mod generation;
mod generation_lifecycle;
mod generation_prompt;
pub mod post_process;
mod records;
mod response_json;
pub mod source_spans;

#[cfg(test)]
mod tests;

pub use generation::ChapterGenerateStreamInput;
pub use generation_prompt::build_chapter_generation_request;
pub use response_json::normalize_generated_chapter_content_layout;

use dashmap::DashMap;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use a4n_core::error::AppError;
use a4n_engine::llm::provider::is_truncated_finish_reason;

type ChapterGenerationKey = (i64, i32);
type ChapterGenerationLock = Arc<AsyncMutex<()>>;
type ChapterGenerationLocks = DashMap<ChapterGenerationKey, ChapterGenerationLock>;

#[derive(Clone)]
pub struct ChapterService {
    db: Arc<DatabaseConnection>,
    generation_locks: Arc<ChapterGenerationLocks>,
}

impl ChapterService {
    pub fn new(
        db: Arc<DatabaseConnection>,
        _event_bus: Arc<dyn a4n_core::event::bridge::EventBus>,
    ) -> Self {
        Self {
            db,
            generation_locks: Arc::new(DashMap::new()),
        }
    }

    pub fn active_generation_chapter(
        &self,
        project_id: i64,
        chapter_numbers: &[i32],
    ) -> Option<i32> {
        chapter_numbers.iter().copied().find(|chapter_number| {
            chapter_generation_lock_is_active(&self.generation_locks, project_id, *chapter_number)
        })
    }
}

fn chapter_generation_lock_is_active(
    generation_locks: &ChapterGenerationLocks,
    project_id: i64,
    chapter_number: i32,
) -> bool {
    generation_locks
        .get(&(project_id, chapter_number))
        .is_some_and(|lock| lock.try_lock().is_err())
}

fn ensure_chapter_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "模型生成{label}时达到当前场景输出上限，返回 JSON 被截断；请在设置页调大章节生成 max_tokens，或缩短章节目标/上下文后重试"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod generation_lock_tests {
    use super::*;

    #[test]
    fn chapter_generation_lock_reports_only_held_locks_as_active() {
        let locks = ChapterGenerationLocks::new();
        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert((7, 3), lock.clone());

        assert!(!chapter_generation_lock_is_active(&locks, 7, 3));
        assert!(!chapter_generation_lock_is_active(&locks, 7, 4));

        let guard = lock.try_lock_owned().unwrap();

        assert!(chapter_generation_lock_is_active(&locks, 7, 3));
        assert!(!chapter_generation_lock_is_active(&locks, 8, 3));

        drop(guard);
        assert!(!chapter_generation_lock_is_active(&locks, 7, 3));
    }
}
