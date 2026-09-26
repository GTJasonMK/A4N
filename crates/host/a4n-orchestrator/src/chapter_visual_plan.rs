use crate::OrchestratorContext;
mod generation;
mod tools;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanRecordInput, SaveChapterVisualPlanDraftInput,
};
use a4n_db::chapter_visual_plan::{
    ChapterVisualPlanDbError, ConfirmChapterVisualPlanOutcome, ParsedChapterVisualPlan,
};
pub(crate) use a4n_domain::shooting_script::{
    DirectorScriptPlan, DirectorScriptUnit, segment_addition_id,
};
use a4n_domain::{chapter::ResolvedChapterSource, chapter_visual_plan::ExpectedVisualPlanDraft};
use a4n_error::AppError;

impl OrchestratorContext {
    pub(crate) async fn save_chapter_visual_plan_draft_for_user_validated(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        let expected = expected_draft(input.record_id, input.expected_revision)?;
        a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: input.authoring_mode,
                expected,
                expected_source_hash: input.expected_source_hash,
                plan: input.plan,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub(crate) async fn confirm_chapter_visual_plan_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmChapterVisualPlanOutcome, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub(crate) async fn discard_chapter_visual_plan_draft_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        a4n_db::chapter_visual_plan::discard_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::DiscardChapterVisualPlanDraftInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub(crate) async fn confirmed_visual_plan_for_shooting_script(
        &self,
        source: &ResolvedChapterSource,
        requested_id: Option<i64>,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            source,
        )
        .await
        .map_err(visual_plan_db_error)?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "第 {} 章缺少已确认的章节视觉编排方案；请先在“导演编排”中确认方案",
                source.chapter_number
            ))
        })?;
        if requested_id.is_some_and(|id| id != confirmed.record.id) {
            return Err(AppError::InvalidArgument(format!(
                "visual_plan_id 不是当前正文的 confirmed 方案: requested={requested_id:?}, current={}",
                confirmed.record.id
            )));
        }
        Ok(confirmed)
    }

    async fn resolve_visual_plan_source(
        &self,
        project_id: i64,
        source_chapter_id: i64,
    ) -> Result<ResolvedChapterSource, AppError> {
        a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            project_id,
            source_chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadApplicationPort
    for OrchestratorContext
{
    async fn preview_point_generation(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::PreviewVisualPlanPointInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::VisualPlanPointGenerationPreview, AppError>
    {
        self.preview_visual_plan_point_generation(user_id, input.project_id, &input.options)
            .await
    }

    async fn prompt_candidate(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ReadVisualPlanPromptCandidateInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::VisualPlanPromptCandidate, AppError> {
        self.read_visual_plan_prompt_candidate(user_id, input).await
    }
    fn project_editor(
        &self,
        plan: &a4n_api_contract::chapter_visual_plan::ChapterVisualPlanV3,
        view: &a4n_api_contract::chapter_visual_plan::VisualPlanReadView,
    ) -> Result<a4n_api_contract::chapter_visual_plan::VisualPlanProjection, AppError> {
        a4n_api_contract::chapter_visual_plan::project_visual_plan(plan, view).map_err(|error| {
            match error {
                a4n_api_contract::chapter_visual_plan::VisualPlanProjectionError::InvalidRequest(
                    message,
                ) => AppError::InvalidArgument(message),
                a4n_api_contract::chapter_visual_plan::VisualPlanProjectionError::NotFound(
                    message,
                ) => AppError::NotFound(message),
            }
        })
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort
    for OrchestratorContext
{
    async fn start_prompt_generation(
        &self,
        user_id: i64,
        input: &a4n_api_contract::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
    ) -> Result<
        a4n_api_contract::generation_tasks::GenerationTaskStart,
        a4n_api_contract::generation_tasks::GenerationTaskStartError,
    > {
        self.start_visual_plan_prompt_generation_task(user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_prompt_generation(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.run_visual_plan_prompt_generation(user_id, input, handle)
            .await
    }
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView, AppError> {
        self.save_chapter_visual_plan_draft_for_user_validated(user_id, input)
            .await
            .map(chapter_visual_plan_view)
    }

    async fn edit_draft(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::EditChapterVisualPlanDraftInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanEditResult, AppError> {
        self.edit_chapter_visual_plan_draft_for_user(user_id, input)
            .await
    }

    async fn confirm(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ConfirmedChapterVisualPlanView, AppError>
    {
        let outcome = self
            .confirm_chapter_visual_plan_for_user_validated(user_id, input)
            .await?;
        Ok(
            a4n_api_contract::chapter_visual_plan::ConfirmedChapterVisualPlanView {
                visual_plan: chapter_visual_plan_view(outcome.visual_plan),
                invalidated_project_ids: outcome.invalidated_project_ids,
                shooting_script_sync: outcome.shooting_script_sync,
            },
        )
    }

    async fn discard(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.discard_chapter_visual_plan_draft_for_user_validated(user_id, input)
            .await
    }
}

fn chapter_visual_plan_view(
    value: ParsedChapterVisualPlan,
) -> a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView {
    a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView {
        record: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordSummary {
            id: value.record.id,
            plan_version: value.record.plan_version,
            revision: value.record.revision,
            status: value.status,
            authoring_mode: value.authoring_mode,
            source_content_sha256: value.record.source_content_sha256,
            plan_sha256: value.record.plan_sha256,
            confirmed_at: value.record.confirmed_at.map(|value| value.to_string()),
        },
        plan: value.plan,
    }
}

fn expected_draft(
    record_id: Option<i64>,
    revision: Option<i32>,
) -> Result<Option<ExpectedVisualPlanDraft>, AppError> {
    match (record_id, revision) {
        (None, None) => Ok(None),
        (Some(record_id), Some(revision)) if record_id > 0 && revision > 0 => {
            Ok(Some(ExpectedVisualPlanDraft {
                record_id,
                revision,
            }))
        }
        _ => Err(AppError::InvalidArgument(
            "record_id 与 expected_revision 必须同时提供且大于 0".into(),
        )),
    }
}

fn ensure_plan_belongs_to_source(
    plan: &ParsedChapterVisualPlan,
    source: &ResolvedChapterSource,
) -> Result<(), AppError> {
    if plan.record.root_project_id != source.root_project_id
        || plan.record.source_chapter_id != source.source_chapter_id
        || plan.record.source_snapshot_id != source.source_snapshot_id
        || plan.record.source_snapshot_chapter_id != source.source_snapshot_chapter_id
    {
        return Err(AppError::InvalidArgument(
            "章节视觉编排与当前章节正文不匹配".into(),
        ));
    }
    Ok(())
}

pub(crate) fn project_visual_plan_for_shooting_script(
    plan: &a4n_domain::chapter_visual_plan::ChapterVisualPlanV3,
    source_spans: &[a4n_domain::source_spans::SourceSpan],
) -> Result<DirectorScriptPlan, AppError> {
    a4n_domain::shooting_script::project_visual_plan_for_shooting_script(plan, source_spans)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

pub(crate) fn visual_plan_db_error(error: ChapterVisualPlanDbError) -> AppError {
    match error {
        ChapterVisualPlanDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterVisualPlanDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "读写章节视觉编排".into(),
        },
        ChapterVisualPlanDbError::NotFound(message) => AppError::NotFound(message),
        ChapterVisualPlanDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => AppError::InvalidState {
            current: format!(
                "visual_plan_revision_conflict:id={record_id},expected={expected_revision}"
            ),
            operation: "更新章节视觉编排".into(),
        },
        ChapterVisualPlanDbError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_identity_must_be_complete_and_positive() {
        assert!(expected_draft(None, None).unwrap().is_none());
        assert!(expected_draft(Some(1), Some(2)).unwrap().is_some());
        assert!(expected_draft(Some(1), None).is_err());
        assert!(expected_draft(None, Some(2)).is_err());
        assert!(expected_draft(Some(0), Some(2)).is_err());
    }
}
