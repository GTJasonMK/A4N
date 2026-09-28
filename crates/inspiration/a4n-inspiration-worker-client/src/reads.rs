use super::InspirationWorkerClient;
use a4n_api_contract::inspiration::{
    InspirationHistoryInput, InspirationHistoryReadApplicationPort, InspirationMessageSummary,
};
use a4n_error::AppError;

#[async_trait::async_trait]
impl InspirationHistoryReadApplicationPort for InspirationWorkerClient {
    async fn history(
        &self,
        user_id: i64,
        input: InspirationHistoryInput,
    ) -> Result<Vec<InspirationMessageSummary>, AppError> {
        self.command(user_id, input).await
    }
}
