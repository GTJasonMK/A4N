use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_patch, optimization_patch_set};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use serde_json::Value;

use super::runtime_tools::EDIT_ARTICLE_TOOL;
use super::{OptimizationPatch, OptimizationPatchProposal, OptimizeService};

const PATCH_SET_STATUS_PENDING: &str = "pending";
const PATCH_SET_STATUS_APPLIED: &str = "applied";
const PATCH_SET_STATUS_REJECTED: &str = "rejected";
const PATCH_DECISION_PENDING: &str = "pending";
const PATCH_DECISION_ACCEPTED: &str = "accepted";
const PATCH_DECISION_REJECTED: &str = "rejected";
const PATCH_APPLY_STATUS_PENDING: &str = "pending";
const PATCH_APPLY_STATUS_APPLIED: &str = "applied";

impl OptimizeService {
    pub async fn list_patch_sets(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        optimization_patch_set::Entity::find()
            .filter(optimization_patch_set::Column::SessionId.eq(session_id))
            .order_by_desc(optimization_patch_set::Column::UpdatedAt)
            .order_by_desc(optimization_patch_set::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_patch_set_patches(
        &self,
        patch_set_id: i64,
    ) -> Result<Vec<optimization_patch::Model>, AppError> {
        optimization_patch::Entity::find()
            .filter(optimization_patch::Column::PatchSetId.eq(patch_set_id))
            .order_by_asc(optimization_patch::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn mark_matching_patch_sets_applied(
        &self,
        session_id: i64,
        base_content_hash: &str,
    ) -> Result<(), AppError> {
        let patch_sets = optimization_patch_set::Entity::find()
            .filter(optimization_patch_set::Column::SessionId.eq(session_id))
            .filter(optimization_patch_set::Column::BaseContentHash.eq(base_content_hash))
            .filter(optimization_patch_set::Column::Status.eq(PATCH_SET_STATUS_PENDING))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        for patch_set in patch_sets {
            self.update_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_APPLIED,
                PATCH_DECISION_ACCEPTED,
                PATCH_APPLY_STATUS_APPLIED,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reject_pending_patch_sets(&self, session_id: i64) -> Result<(), AppError> {
        let patch_sets = optimization_patch_set::Entity::find()
            .filter(optimization_patch_set::Column::SessionId.eq(session_id))
            .filter(optimization_patch_set::Column::Status.eq(PATCH_SET_STATUS_PENDING))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        for patch_set in patch_sets {
            self.update_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_REJECTED,
                PATCH_DECISION_REJECTED,
                PATCH_APPLY_STATUS_PENDING,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn persist_patch_tool_result(
        &self,
        turn_id: i64,
        session_id: i64,
        project_id: i64,
        chapter_id: i64,
        result: &Value,
    ) -> Result<Option<optimization_patch_set::Model>, AppError> {
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

    pub async fn persist_patch_proposal(
        &self,
        turn_id: i64,
        proposal: &OptimizationPatchProposal,
    ) -> Result<Option<optimization_patch_set::Model>, AppError> {
        if proposal.patches.is_empty() {
            return Ok(None);
        }

        let now = Utc::now().naive_utc();
        let patch_set = optimization_patch_set::ActiveModel {
            turn_id: Set(turn_id),
            session_id: Set(proposal.session_id),
            chapter_id: Set(proposal.chapter_id),
            base_content_hash: Set(proposal.base_content_hash.clone()),
            status: Set(PATCH_SET_STATUS_PENDING.to_string()),
            summary: Set(proposal.summary.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        for patch in &proposal.patches {
            optimization_patch::ActiveModel {
                patch_set_id: Set(patch_set.id),
                patch_id: Set(patch.patch_id.clone()),
                issue_id: Set(patch.issue_id.clone()),
                operation: Set(patch.operation.clone()),
                old_text: Set(patch.old_text.clone()),
                new_text: Set(patch.new_text.clone()),
                reason: Set(patch.reason.clone()),
                risk: Set(patch.risk.clone()),
                decision: Set(PATCH_DECISION_PENDING.to_string()),
                apply_status: Set(PATCH_APPLY_STATUS_PENDING.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        }

        Ok(Some(patch_set))
    }

    async fn update_patch_set_status(
        &self,
        patch_set: optimization_patch_set::Model,
        patch_set_status: &str,
        patch_decision: &str,
        patch_apply_status: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        let patches = self.list_patch_set_patches(patch_set.id).await?;
        for patch in patches {
            let mut active: optimization_patch::ActiveModel = patch.into();
            active.decision = Set(patch_decision.to_string());
            active.apply_status = Set(patch_apply_status.to_string());
            active.updated_at = Set(now);
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
        }

        let mut active: optimization_patch_set::ActiveModel = patch_set.into();
        active.status = Set(patch_set_status.to_string());
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
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
