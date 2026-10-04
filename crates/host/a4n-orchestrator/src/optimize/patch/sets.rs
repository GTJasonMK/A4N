use a4n_db::optimize::{OptimizationPatchCreateInput, OptimizationPatchSetCreateInput};
use a4n_domain::project::OptimizationPatchSetRecord;
use a4n_error::AppError;

use serde_json::Value;

use super::super::runtime_tools::EDIT_ARTICLE_TOOL;
use super::super::{
    OptimizationPatch, OptimizationPatchProposal, OrchestratorContext, optimize_db_error,
};

impl OrchestratorContext {
    pub(in crate::optimize) async fn persist_patch_tool_result(
        &self,
        turn_id: i64,
        session_id: i64,
        project_id: i64,
        chapter_id: i64,
        result: &Value,
    ) -> Result<Option<OptimizationPatchSetRecord>, AppError> {
        if result.get("tool").and_then(Value::as_str) != Some(EDIT_ARTICLE_TOOL)
            || result.get("mode").and_then(Value::as_str) != Some("pending_confirmation")
        {
            return Ok(None);
        }

        let patches_value = result
            .get("patches")
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("edit_article 结果缺少 patches".into()))?;
        let patches: Vec<OptimizationPatch> = serde_json::from_value(patches_value)?;
        let proposal = OptimizationPatchProposal {
            session_id,
            project_id,
            chapter_id,
            base_content_hash: required_result_string(result, "base_content_hash")?,
            summary: required_result_string(result, "summary")?,
            patches,
        };
        self.persist_patch_proposal(turn_id, &proposal).await
    }

    pub(in crate::optimize) async fn persist_patch_proposal(
        &self,
        turn_id: i64,
        proposal: &OptimizationPatchProposal,
    ) -> Result<Option<OptimizationPatchSetRecord>, AppError> {
        if let Some(existing) =
            a4n_db::optimize::optimization_patch_set_by_turn_id(self.database.executor(), turn_id)
                .await
                .map_err(optimize_db_error)?
        {
            return Ok(Some(existing));
        }
        if proposal.patches.is_empty() {
            return Ok(None);
        }

        a4n_db::optimize::create_optimization_patch_set(
            self.database.executor(),
            OptimizationPatchSetCreateInput {
                turn_id,
                session_id: proposal.session_id,
                chapter_id: proposal.chapter_id,
                base_content_hash: proposal.base_content_hash.clone(),
                status: a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_PENDING.to_string(),
                summary: proposal.summary.clone(),
                patch_decision: a4n_domain::project::OPTIMIZATION_PATCH_DECISION_PENDING
                    .to_string(),
                patch_apply_status: a4n_domain::project::OPTIMIZATION_PATCH_APPLY_STATUS_PENDING
                    .to_string(),
                patches: proposal
                    .patches
                    .iter()
                    .map(|patch| OptimizationPatchCreateInput {
                        patch_id: patch.patch_id.clone(),
                        issue_id: patch.issue_id.clone(),
                        operation: patch.operation.as_str().to_string(),
                        old_text: patch.old_text.clone(),
                        new_text: patch.new_text.clone(),
                        reason: patch.reason.clone(),
                        risk: patch.risk.clone(),
                    })
                    .collect(),
            },
        )
        .await
        .map_err(optimize_db_error)
    }
}

fn required_result_string(result: &Value, field: &str) -> Result<String, AppError> {
    result
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("edit_article 结果缺少 {field}")))
}
