use super::*;

#[async_trait::async_trait]
impl PortraitRecordMutationApplicationPort for PortraitWorkerClient {
    async fn set_primary(&self, user_id: i64, portrait_id: i64) -> Result<(), AppError> {
        self.command(
            user_id,
            None,
            PORTRAIT_SET_PRIMARY_OPERATION,
            PortraitRecordInput { portrait_id },
        )
        .await
    }
}
