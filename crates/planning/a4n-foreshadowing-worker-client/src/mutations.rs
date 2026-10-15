use super::*;

#[async_trait::async_trait]
impl ForeshadowingMutationApplicationPort for ForeshadowingWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        host_port: &dyn ForeshadowingRagSyncHostPort,
        input: CreateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        let result: ForeshadowingMutationResult = self
            .command(
                user_id,
                input.project_id,
                FORESHADOWING_CREATE_OPERATION,
                CreateInput(input),
            )
            .await?;
        let record = result
            .record
            .ok_or_else(|| AppError::ConfigError("伏笔 worker 创建响应缺少已提交记录".into()))?;
        Ok(sync_warning(
            Some(record),
            host_port
                .sync_record(result.project_id, result.foreshadowing_id)
                .await,
            result.project_id,
            result.foreshadowing_id,
            "伏笔已保存，但检索索引更新失败，可稍后重试",
        ))
    }

    async fn update(
        &self,
        user_id: i64,
        host_port: &dyn ForeshadowingRagSyncHostPort,
        input: UpdateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        let result: ForeshadowingMutationResult = self
            .command(
                user_id,
                input.project_id,
                FORESHADOWING_UPDATE_OPERATION,
                UpdateInput(input),
            )
            .await?;
        let record = result
            .record
            .ok_or_else(|| AppError::ConfigError("伏笔 worker 更新响应缺少已提交记录".into()))?;
        Ok(sync_warning(
            Some(record),
            host_port
                .sync_record(result.project_id, result.foreshadowing_id)
                .await,
            result.project_id,
            result.foreshadowing_id,
            "伏笔已保存，但检索索引更新失败，可稍后重试",
        ))
    }

    async fn delete(
        &self,
        user_id: i64,
        host_port: &dyn ForeshadowingRagSyncHostPort,
        input: DeleteForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        let result: ForeshadowingMutationResult = self
            .command(
                user_id,
                input.project_id,
                FORESHADOWING_DELETE_OPERATION,
                DeleteInput(input),
            )
            .await?;
        Ok(sync_warning(
            None,
            host_port
                .delete_record(result.project_id, result.foreshadowing_id)
                .await,
            result.project_id,
            result.foreshadowing_id,
            "伏笔已删除，但检索索引清理失败，可稍后重试",
        ))
    }
}
