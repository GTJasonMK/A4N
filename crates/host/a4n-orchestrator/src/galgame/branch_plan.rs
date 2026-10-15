use a4n_db::media_authoring as records;
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentRecord,
    AuthoringDocumentStatus,
};
use a4n_error::AppError;

use crate::{OrchestratorContext, media_authoring::authoring_error};

use super::GalgamePipelinePromptContext;

pub(crate) struct GalgameBranchPlanCandidate {
    pub inputs: AuthoringDocumentInputs,
    pub editable: Option<AuthoringDocumentRecord>,
}

impl GalgameBranchPlanCandidate {
    pub fn matching(&self) -> Option<&AuthoringDocumentRecord> {
        self.editable
            .as_ref()
            .filter(|record| record.inputs == self.inputs)
    }
}

pub(crate) async fn galgame_pipeline_branch_plan_candidate(
    service: &OrchestratorContext,
    project_id: i64,
    context: &GalgamePipelinePromptContext,
    allow_draft_adaptations: bool,
) -> Result<GalgameBranchPlanCandidate, AppError> {
    let db = service.database.executor();
    let mut inputs = AuthoringDocumentInputs {
        sources: Vec::new(),
        dependencies: Vec::new(),
        video_reference: None,
        preprocessing_fingerprint: None,
    };
    for adaptations in context.adaptations.chunks(records::AUTHORING_READ_BATCH_SIZE) {
        let record_ids = adaptations
            .iter()
            .map(|adaptation| adaptation.adaptation_plan_id)
            .collect::<Vec<_>>();
        let rows = records::authoring_document_rows_by_ids(db, project_id, &record_ids)
            .await
            .map_err(authoring_error)?;
        for (adaptation, row) in adaptations.iter().zip(rows) {
            let row = row.map_err(authoring_error)?.ok_or_else(|| {
                authoring_error(records::AuthoringDocumentError::NotFound(
                    adaptation.adaptation_plan_id,
                ))
            })?;
            let record = row.parse().map_err(authoring_error)?;
            let status_is_usable =
                adaptation_status_is_usable(record.status, allow_draft_adaptations);
            if !status_is_usable || record.revision != adaptation.adaptation_plan_revision {
                return Err(AppError::InvalidArgument(
                    "改编计划版本已变化，请重新准备分支阶段".into(),
                ));
            }
            inputs.sources.extend(record.inputs.sources.iter().cloned());
            inputs.dependencies.push(record.reference());
        }
    }
    let editable = records::editable_authoring_document(
        db,
        project_id,
        context.source_snapshot_id,
        None,
        AuthoringDocumentKind::GalgameBranchPlan,
    )
    .await
    .map_err(authoring_error)?;
    Ok(GalgameBranchPlanCandidate { inputs, editable })
}

fn adaptation_status_is_usable(status: AuthoringDocumentStatus, allow_draft: bool) -> bool {
    status == AuthoringDocumentStatus::Adopted
        || (allow_draft && status == AuthoringDocumentStatus::Draft)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::media_authoring::AuthoringDocumentRef;
    use serde_json::json;

    fn inputs(revision: i32) -> AuthoringDocumentInputs {
        AuthoringDocumentInputs {
            sources: Vec::new(),
            video_reference: None,
            dependencies: vec![AuthoringDocumentRef {
                id: 8,
                revision,
                sha256: "plan-hash".into(),
            }],
            preprocessing_fingerprint: None,
        }
    }

    #[test]
    fn branch_plan_is_reusable_only_for_the_current_adaptation_revision() {
        let expected = inputs(2);
        let candidate = GalgameBranchPlanCandidate {
            inputs: expected.clone(),
            editable: Some(AuthoringDocumentRecord {
                id: 12,
                project_id: 4,
                source_snapshot_id: 6,
                source_chapter_id: None,
                kind: AuthoringDocumentKind::GalgameBranchPlan,
                version: 1,
                revision: 3,
                status: AuthoringDocumentStatus::Draft,
                inputs: expected,
                document: json!({"choices": []}),
                sha256: "branch-hash".into(),
            }),
        };

        assert!(candidate.matching().is_some());
        let mut stale = candidate;
        stale.inputs = inputs(1);
        assert!(stale.matching().is_none());
    }

    #[test]
    fn preview_accepts_a_saved_draft_that_execution_adopts_before_the_stage() {
        assert!(adaptation_status_is_usable(
            AuthoringDocumentStatus::Adopted,
            false
        ));
        assert!(!adaptation_status_is_usable(
            AuthoringDocumentStatus::Draft,
            false
        ));
        assert!(adaptation_status_is_usable(
            AuthoringDocumentStatus::Draft,
            true
        ));
        assert!(!adaptation_status_is_usable(
            AuthoringDocumentStatus::Superseded,
            true
        ));
    }
}
