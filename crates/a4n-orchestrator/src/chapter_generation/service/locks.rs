use super::ChapterGenerationLocks;

pub(super) fn chapter_generation_lock_is_active(
    generation_locks: &ChapterGenerationLocks,
    project_id: i64,
    chapter_number: i32,
) -> bool {
    generation_locks
        .get(&(project_id, chapter_number))
        .is_some_and(|lock| lock.try_lock().is_err())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex as AsyncMutex;

    use super::*;

    #[test]
    fn reports_only_held_matching_locks_as_active() {
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
