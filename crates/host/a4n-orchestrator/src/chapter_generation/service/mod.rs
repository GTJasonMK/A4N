//! 章节生成服务：核心写作引擎
//!
//! 对应文档 05-章节生成系统

pub(in crate::chapter_generation) mod generation {
    pub(super) mod lifecycle;

    pub(in crate::chapter_generation) mod checkpoint;
}

mod response_json;

#[cfg(test)]
mod tests;

pub(in crate::chapter_generation) use generation::lifecycle::{
    ChapterGenerationSession, ChapterGenerationTrace,
};

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use a4n_db::DbHandle;

type ChapterGenerationKey = (i64, i32);
type ChapterGenerationLock = Arc<AsyncMutex<()>>;
type ChapterGenerationLocks = DashMap<ChapterGenerationKey, ChapterGenerationLock>;

#[derive(Clone)]
pub struct ChapterGenerationService {
    db: DbHandle,
    generation_locks: Arc<ChapterGenerationLocks>,
}

impl ChapterGenerationService {
    pub fn new(db: DbHandle) -> Self {
        Self {
            db,
            generation_locks: Arc::new(DashMap::new()),
        }
    }
}
