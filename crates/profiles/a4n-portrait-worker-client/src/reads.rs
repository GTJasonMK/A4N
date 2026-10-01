use super::*;

#[async_trait::async_trait]
impl PortraitRecordReadApplicationPort for PortraitWorkerClient {
    async fn list_by_project(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PortraitSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            PORTRAIT_LIST_OPERATION,
            PortraitProjectInput { project_id },
        )
        .await
    }

    async fn primary_by_profile(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<Option<PortraitSummary>, AppError> {
        let response: PortraitLookupResponse = self
            .command(
                user_id,
                None,
                PORTRAIT_PRIMARY_OPERATION,
                PortraitProfileInput { profile_id },
            )
            .await?;
        Ok(response.portrait)
    }

    async fn by_id(
        &self,
        user_id: i64,
        portrait_id: i64,
    ) -> Result<Option<PortraitSummary>, AppError> {
        let response: PortraitLookupResponse = self
            .command(
                user_id,
                None,
                PORTRAIT_BY_ID_OPERATION,
                PortraitRecordInput { portrait_id },
            )
            .await?;
        Ok(response.portrait)
    }

    async fn missing_generation_seeds(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PortraitGenerationSeed>, AppError> {
        let seeds: Vec<PortraitGenerationSeedWire> = self
            .command(
                user_id,
                Some(project_id),
                PORTRAIT_MISSING_SEEDS_OPERATION,
                PortraitProjectInput { project_id },
            )
            .await?;
        Ok(seeds
            .into_iter()
            .map(|seed| PortraitGenerationSeed {
                profile_name: seed.profile_name,
                prompt: seed.prompt,
            })
            .collect())
    }
}
