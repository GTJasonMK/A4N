use super::super::errors::visual_plan_error;
use super::super::helpers::*;
use super::visual_plan_view;
use crate::*;

impl WorkerState {
    pub(in crate::worker) async fn save_visual_plan_draft(
        &self,
        request: WorkerEnvelope<SaveChapterVisualPlanDraftInput>,
    ) -> Result<ChapterVisualPlanView, WorkerFailure> {
        validate_request(&request, SAVE_VISUAL_PLAN_DRAFT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_optional_target_revision(&request, request.payload.expected_revision)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        let expected = match (request.payload.record_id, request.payload.expected_revision) {
            (None, None) => None,
            (Some(record_id), Some(revision)) if record_id > 0 && revision > 0 => {
                Some(a4n_domain::chapter_visual_plan::ExpectedVisualPlanDraft {
                    record_id,
                    revision,
                })
            }
            _ => {
                return Err(WorkerFailure::invalid_request(
                    "record_id 与 expected_revision 必须同时提供且大于 0",
                ));
            }
        };
        let operation_id = request
            .payload
            .operation_id
            .unwrap_or(request.idempotency_key);
        let record = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft_with_operation(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: request.payload.authoring_mode,
                expected,
                expected_source_hash: request.payload.expected_source_hash,
                plan: request.payload.plan,
            },
            &operation_id,
        )
        .await
        .map_err(visual_plan_error)?;
        Ok(visual_plan_view(record))
    }

    pub(in crate::worker) async fn confirm_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanRecordInput>,
    ) -> Result<ConfirmedChapterVisualPlanView, WorkerFailure> {
        validate_request(&request, CONFIRM_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        ensure_visual_plan_belongs_to_source(self, &request.payload).await?;
        let outcome = a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_error)?;
        Ok(ConfirmedChapterVisualPlanView {
            visual_plan: visual_plan_view(outcome.visual_plan),
            invalidated_project_ids: outcome.invalidated_project_ids,
            shooting_script_sync: outcome.shooting_script_sync,
        })
    }

    pub(in crate::worker) async fn discard_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanRecordInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, DISCARD_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        ensure_visual_plan_belongs_to_source(self, &request.payload).await?;
        a4n_db::chapter_visual_plan::discard_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::DiscardChapterVisualPlanDraftInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_error)
    }
}

async fn ensure_visual_plan_belongs_to_source(
    state: &WorkerState,
    input: &ChapterVisualPlanRecordInput,
) -> Result<(), WorkerFailure> {
    let source = resolve_source(state, input.project_id, input.source_chapter_id).await?;
    let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
        state.database.connection(),
        input.record_id,
    )
    .await
    .map_err(visual_plan_error)?;
    if plan.record.project_id != source.requested_project_id
        || plan.record.root_project_id != source.root_project_id
        || plan.record.source_chapter_id != source.source_chapter_id
        || plan.record.source_snapshot_id != source.source_snapshot_id
        || plan.record.source_snapshot_chapter_id != source.source_snapshot_chapter_id
    {
        return Err(WorkerFailure::invalid_request(
            "章节视觉编排与当前章节正文不匹配",
        ));
    }
    Ok(())
}
