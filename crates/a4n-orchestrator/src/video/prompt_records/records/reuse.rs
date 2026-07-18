use std::collections::BTreeMap;

use crate::video::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::PromptOutputLanguage;

use super::super::contracts::{
    PromptGenerationContextForWorkflow, PromptWorkflowRowsContext, PromptWorkflowRowsQuery,
};
use super::super::matching::prompt_matches_generation_context_for_workflow;

impl OrchestratorContext {
    pub(in crate::video) async fn reusable_prompt_rows(
        &self,
        context: PromptWorkflowRowsContext<'_>,
        prompt_output_language: PromptOutputLanguage,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let rows = self
            .prompt_rows_matching_workflow(PromptWorkflowRowsQuery {
                context,
                prompt_output_language,
                exclude_running: true,
            })
            .await?;

        let mut by_shot = BTreeMap::<i32, chapter_video_prompt::Model>::new();
        for row in rows {
            by_shot.entry(row.shot_number).or_insert(row);
        }
        Ok(by_shot.into_values().collect())
    }

    pub(in crate::video) async fn matching_prompt_rows_for_workflow(
        &self,
        context: PromptWorkflowRowsContext<'_>,
        prompt_output_language: PromptOutputLanguage,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.prompt_rows_matching_workflow(PromptWorkflowRowsQuery {
            context,
            prompt_output_language,
            exclude_running: false,
        })
        .await
    }

    async fn prompt_rows_matching_workflow(
        &self,
        query: PromptWorkflowRowsQuery<'_>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let PromptWorkflowRowsQuery {
            context,
            prompt_output_language,
            exclude_running,
        } = query;
        let rows = a4n_db::video::chapter_video_prompt_workflow_records(
            self.db.connection(),
            context.project_id,
            context.chapter_id,
            context.prompt_type,
            context.workflow_mode,
            exclude_running,
        )
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .filter(|row| {
                row.duration_secs == context.duration_secs
                    && prompt_matches_generation_context_for_workflow(
                        row,
                        PromptGenerationContextForWorkflow {
                            expected_workflow_mode: context.workflow_mode,
                            prompt_type: context.prompt_type,
                            source_metadata: context.source_metadata,
                            style: context.style,
                            aspect_ratio: context.aspect_ratio,
                            reference_ids: context.reference_ids,
                            generation_asset_fingerprint: context.generation_asset_fingerprint,
                            source_prompt_type: context.source_prompt_type,
                            video_execution_identity: context.video_execution_identity,
                            prompt_output_language,
                        },
                    )
            })
            .collect())
    }
}
