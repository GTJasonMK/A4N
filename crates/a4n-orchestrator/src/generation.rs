pub use a4n_db::generation::{
    AttachGenerationRunsToWorkflowInput, GENERATION_ENTITY_KINDS, GENERATION_ENTITY_ROLES,
    GENERATION_MEDIA_REF_ROLES, GENERATION_PROMPT_KINDS, GENERATION_SPEC_STATUSES,
    GENERATION_SPEC_TARGET_ENTITY_KINDS, GENERATION_SPEC_TARGET_KINDS,
    GENERATION_TEXT_ANCHOR_ROLES, GenerationRunCheckpointContext, GenerationRunTraceTarget,
    LlmCallUsageTraceInput, PromptArtifactInsertInput, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, RecordLlmCallTraceInput,
    RecordStartedGenerationRunInput, SPEC_STATUS_DRAFT, generation_checkpoint_fingerprint_sha256,
    is_valid_generation_entity_kind, is_valid_generation_entity_role,
    is_valid_generation_media_ref_role, is_valid_generation_prompt_kind,
    is_valid_generation_spec_status, is_valid_generation_spec_target_entity_kind,
    is_valid_generation_spec_target_kind, is_valid_generation_text_anchor_role, render_prompt_text,
};

use a4n_core::error::AppError;
use a4n_core::models::generation::{generation_run, llm_call_trace};
use serde_json::Value;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn record_completed_generation_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generation_run::Model, AppError> {
        a4n_db::generation::record_completed_run_with_checkpoint_transaction(
            self.db.connection(),
            input,
        )
        .await
        .map_err(generation_error)
    }

    pub async fn attach_generation_runs_to_workflow_for_target(
        &self,
        input: AttachGenerationRunsToWorkflowInput,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        a4n_db::generation::attach_runs_to_workflow_for_target(self.db.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub async fn record_llm_call_trace(
        &self,
        input: RecordLlmCallTraceInput,
    ) -> Result<llm_call_trace::Model, AppError> {
        a4n_db::generation::record_llm_call_trace(self.db.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub async fn complete_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        response_summary_json: Option<Value>,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        a4n_db::generation::complete_runs_for_target(
            self.db.connection(),
            target_table,
            target_id,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }

    pub async fn fail_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        error_message: String,
        response_summary_json: Option<Value>,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        a4n_db::generation::fail_runs_for_target(
            self.db.connection(),
            target_table,
            target_id,
            error_message,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }
}

pub(crate) fn generation_error(error: a4n_db::generation::GenerationDbError) -> AppError {
    match error {
        a4n_db::generation::GenerationDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::generation::GenerationDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::generation::GenerationDbError::Database(error) => AppError::Database(error),
    }
}
