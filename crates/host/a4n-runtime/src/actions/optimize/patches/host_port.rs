use a4n_error::AppError;
use a4n_optimize_host_contract::OptimizePatchApplyHostPort;
use async_trait::async_trait;

use crate::AppState;

pub(super) struct RuntimeOptimizePatchApplyHostPort<'a> {
    state: &'a AppState,
}

impl<'a> RuntimeOptimizePatchApplyHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl OptimizePatchApplyHostPort for RuntimeOptimizePatchApplyHostPort<'_> {
    async fn settle_chapter_content_sync(&self, workflow_run_id: i64) -> Result<(), AppError> {
        let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(self.state);
        self.state
            .chapter_content_sync_application()
            .settle(&host_port, workflow_run_id)
            .await
    }
}
