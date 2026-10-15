#[cfg(test)]
mod performance_tests;
mod ports;
mod request;
mod run;

use crate::OrchestratorContext;
use a4n_api_contract::fact_feedback::{MaterialFactChangeSummary, ProjectFactFeedbackSummary};
use a4n_domain::{
    collaboration::ProjectAccessRole,
    fact_feedback::{FactSourceRevision, pending_fact_sources},
};
use a4n_error::AppError;

pub(crate) const FACT_CONSOLIDATION_KIND: &str = "project_fact_consolidation";
pub(crate) const FACT_REBUILD_KIND: &str = "project_fact_rebuild";
pub(crate) const FACT_CONSOLIDATION_PROMPT: &str = "project_fact_consolidation";
pub(crate) use a4n_domain::fact_feedback::FACT_APPLY_STEP;
pub(crate) use a4n_domain::fact_feedback::{
    CHAPTER_FACT_FEEDBACK_STEP as CHAPTER_FEEDBACK_STEP,
    CHAPTER_FACT_INDEX_STEP as CHAPTER_INDEX_STEP,
};

pub(crate) fn feedback_error(error: a4n_db::fact_feedback::FactFeedbackDbError) -> AppError {
    match error {
        a4n_db::fact_feedback::FactFeedbackDbError::Database(error) => AppError::database(error),
        a4n_db::fact_feedback::FactFeedbackDbError::Json(error) => AppError::Serialization(error),
        a4n_db::fact_feedback::FactFeedbackDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        error => AppError::InvalidState {
            current: "body_fact_feedback".into(),
            operation: error.to_string(),
        },
    }
}

fn business_key(project_id: i64, kind: &str) -> String {
    format!("{kind}:{project_id}")
}

impl OrchestratorContext {
    pub(crate) async fn fact_feedback_warnings(
        &self,
        project_id: i64,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .latest_workflow_run_by_business_key(&business_key(project_id, FACT_CONSOLIDATION_KIND))
            .await?
            .filter(|run| matches!(run.status.as_str(), "failed" | "interrupted" | "cancelled"))
            .and_then(|run| run.error_message)
            .map(|error| vec![format!("逐章事实已保存，但项目事实归纳未完成：{error}")])
            .unwrap_or_default())
    }

    pub(crate) async fn fact_feedback_summary_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectFactFeedbackSummary, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let db = self.database.executor();
        let (config, source_index, changes) = tokio::join!(
            a4n_db::fact_feedback::feedback_config(db, project_id),
            a4n_db::fact_feedback::current_feedback_summary_sources(db, project_id),
            a4n_db::fact_feedback::recent_feedback_changes(db, project_id, 100),
        );
        let config = config?;
        let source_index = source_index.map_err(feedback_error)?;
        let revisions = source_index.revisions;
        let consumed: Vec<FactSourceRevision> =
            serde_json::from_value(config.consolidated_sources_json)?;
        let pending_chapters = pending_fact_sources(&revisions, &consumed)
            .iter()
            .map(|source| source.chapter_number)
            .collect();
        let consolidated_chapters = consumed
            .iter()
            .filter(|source| revisions.contains(source))
            .map(|source| source.chapter_number)
            .collect();
        let latest = [
            self.latest_workflow_run_by_business_key(&business_key(
                project_id,
                FACT_CONSOLIDATION_KIND,
            ))
            .await?,
            self.latest_workflow_run_by_business_key(&business_key(project_id, FACT_REBUILD_KIND))
                .await?,
        ]
        .into_iter()
        .flatten()
        .max_by_key(|run| (run.status == "running", run.updated_at.clone(), run.id));
        Ok(ProjectFactFeedbackSummary {
            project_id,
            interval_chapters: config.interval_chapters,
            pending_chapters,
            consolidated_chapters,
            legacy_chapters: source_index
                .available_chapters
                .into_iter()
                .filter(|chapter| {
                    !revisions
                        .iter()
                        .any(|source| source.chapter_id == chapter.chapter_id)
                })
                .map(|chapter| chapter.chapter_number)
                .collect(),
            last_consolidated_at: config
                .last_consolidated_at
                .map(|time| time.and_utc().to_rfc3339()),
            workflow_run_id: latest.as_ref().map(|run| run.id),
            workflow_status: latest.as_ref().map(|run| run.status.clone()),
            workflow_kind: latest.as_ref().map(|run| run.workflow_kind.clone()),
            can_resume: latest.as_ref().is_some_and(|run| run.can_resume),
            error: latest.and_then(|run| run.error_message),
            changes: changes?
                .into_iter()
                .map(|change| MaterialFactChangeSummary {
                    id: change.id,
                    material_kind: change.material_kind,
                    target_id: change.target_id,
                    field_path: change.field_path,
                    before: change.before_json,
                    after: change.after_json,
                    evidence: change.evidence_json,
                    reason: change.reason,
                    status: change.status,
                    created_at: change.created_at.and_utc().to_rfc3339(),
                })
                .collect(),
        })
    }
}
