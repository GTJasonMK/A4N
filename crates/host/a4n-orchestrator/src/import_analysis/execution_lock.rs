use std::sync::Arc;

use a4n_error::AppError;
use dashmap::DashMap;
use tokio::sync::{Mutex, OwnedMutexGuard};

type ProjectLock = Arc<Mutex<()>>;

#[derive(Clone, Default)]
pub(crate) struct ImportAnalysisExecutionLocks {
    by_project: Arc<DashMap<i64, ProjectLock>>,
}

pub(crate) struct ImportAnalysisExecutionGuard {
    _guard: OwnedMutexGuard<()>,
}

impl ImportAnalysisExecutionLocks {
    pub(crate) fn try_acquire(
        &self,
        project_id: i64,
    ) -> Result<ImportAnalysisExecutionGuard, AppError> {
        let lock = self
            .by_project
            .entry(project_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let guard = lock.try_lock_owned().map_err(|_| AppError::InvalidState {
            current: "analyzing".to_string(),
            operation: "运行导入项目分析".to_string(),
        })?;
        Ok(ImportAnalysisExecutionGuard { _guard: guard })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn analysis_lock_is_per_project_and_released_after_completion() {
        let locks = ImportAnalysisExecutionLocks::default();
        let first_project = locks
            .try_acquire(1)
            .expect("first run should acquire its lock");

        assert!(matches!(
            locks.try_acquire(1),
            Err(AppError::InvalidState { current, .. }) if current == "analyzing"
        ));
        assert!(locks.try_acquire(2).is_ok());

        drop(first_project);
        assert!(locks.try_acquire(1).is_ok());
    }
}
