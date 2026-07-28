use a4n_core::error::AppError;
use a4n_orchestrator::optimize::OptimizePatchApplyHostPort;
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
    async fn postprocess_chapter_after_content_change(
        &self,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(self.state);
        self.state
            .chapter_content()
            .postprocess_chapter_after_content_change_validated(&host_port, chapter_id)
            .await
    }
}
