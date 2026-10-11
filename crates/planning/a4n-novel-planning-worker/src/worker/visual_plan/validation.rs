use super::super::errors::visual_plan_error;
use super::super::helpers::*;
use super::{
    ensure_visual_plan_belongs_to_source_record, visual_plan_resource,
    visual_plan_revision_conflict,
};
use crate::*;

impl WorkerState {
    pub(in crate::worker) async fn validate_visual_plan(
        &self,
        request: WorkerEnvelope<ValidateChapterVisualPlanInput>,
    ) -> Result<ChapterVisualPlanValidationResult, WorkerFailure> {
        validate_request(&request, VALIDATE_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
            self.database.connection(),
            request.payload.record_id,
        )
        .await
        .map_err(visual_plan_error)?;
        ensure_visual_plan_belongs_to_source_record(&plan, &source)?;
        if plan.record.revision != request.payload.expected_revision {
            return Err(visual_plan_revision_conflict(
                plan.record.id,
                request.payload.expected_revision,
            ));
        }
        let result = match request.payload.mode {
            a4n_api_contract::chapter_visual_plan::VisualPlanValidationMode::Draft => {
                plan.plan.validate_draft()
            }
            a4n_api_contract::chapter_visual_plan::VisualPlanValidationMode::Confirmed => {
                plan.plan.validate_confirmed()
            }
        };
        let validation = a4n_api_contract::chapter_visual_plan::VisualPlanValidationResult {
            mode: request.payload.mode,
            valid: result.is_ok(),
            diagnostics: result
                .err()
                .into_iter()
                .map(
                    |error| a4n_api_contract::chapter_visual_plan::VisualPlanDiagnostic {
                        path: error.path,
                        message: error.message,
                    },
                )
                .collect(),
        };
        Ok(ChapterVisualPlanValidationResult {
            resource: visual_plan_resource(&plan),
            validation,
        })
    }
}
