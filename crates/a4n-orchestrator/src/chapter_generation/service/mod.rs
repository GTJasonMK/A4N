//! 章节生成服务：核心写作引擎
//!
//! 对应文档 05-章节生成系统

pub(in crate::chapter_generation) mod generation;
mod generation_lifecycle;
mod locks;
mod records;
mod response_json;

#[cfg(test)]
mod tests;

pub(in crate::chapter_generation) use generation_lifecycle::{
    ChapterGenerationSession, ChapterGenerationTrace,
};

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use a4n_db::SharedDatabaseConnection;
use locks::chapter_generation_lock_is_active;

type ChapterGenerationKey = (i64, i32);
type ChapterGenerationLock = Arc<AsyncMutex<()>>;
type ChapterGenerationLocks = DashMap<ChapterGenerationKey, ChapterGenerationLock>;

#[derive(Clone)]
pub struct ChapterGenerationService {
    db: SharedDatabaseConnection,
    generation_locks: Arc<ChapterGenerationLocks>,
}

impl ChapterGenerationService {
    pub fn new(db: SharedDatabaseConnection) -> Self {
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
