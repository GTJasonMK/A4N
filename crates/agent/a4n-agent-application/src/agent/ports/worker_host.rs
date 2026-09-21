//! Agent worker host command port。

use super::*;

#[async_trait::async_trait]
impl AgentWorkerHostCommandApplicationPort for AgentApplication {
    async fn authorize_worker_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        self.authorize_worker_host_command(request).await
    }
}
