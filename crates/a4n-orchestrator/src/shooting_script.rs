mod assembly;
mod context;
mod contracts;
pub(crate) mod errors;
mod generation;
mod preparation;
mod progress;
mod stage;
mod workflow_rules;

pub use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, EditChapterShootingScriptInput,
    EditedChapterShootingScriptView, GenerateChapterShootingScriptInput,
};
#[cfg(test)]
pub(crate) use contracts::visual_plan_fixture;
pub use contracts::{
    CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptUnitProgress,
    EditedChapterShootingScript, ResolvedChapterShootingScript, ShootingScriptProgressReporter,
    ShootingScriptStreamReporter,
};
pub(crate) use workflow_rules::chapter_shooting_script_workflow_run_input;

use a4n_api_contract::shooting_script::{
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
};
use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

/// Host-facing boundary for chapter shooting-script records and generation.
///
/// Source resolution, visual-plan consistency, workflow state, persistence and
/// API projection stay in this owner. Runtime only supplies progress, stream
/// and cancellation callbacks for long-running generation.
#[async_trait::async_trait]
pub trait ShootingScriptApplicationPort: Send + Sync {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, AppError>;

    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, AppError>;

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, AppError>;

    async fn generate(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptView, AppError>;

    async fn generate_next(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptUnitProgressView, AppError>;
}

#[async_trait::async_trait]
impl ShootingScriptApplicationPort for crate::OrchestratorContext {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, AppError> {
        self.current_chapter_shooting_script_for_user_validated(user_id, input)
            .await
            .map(|resolved| resolved.map(chapter_shooting_script_view))
    }

    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, AppError> {
        self.current_chapter_shooting_script_unit_progress_for_user_validated(user_id, input)
            .await
            .map(|progress| progress.map(chapter_shooting_script_unit_progress_view))
    }

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, AppError> {
        self.edit_current_chapter_shooting_script_for_user_validated(user_id, input)
            .await
            .map(edited_chapter_shooting_script_view)
    }

    async fn generate(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptView, AppError> {
        self.generate_chapter_shooting_script_for_user_validated(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map(chapter_shooting_script_view)
    }

    async fn generate_next(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptUnitProgressView, AppError> {
        self.generate_next_chapter_shooting_script_unit_for_user_validated(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map(chapter_shooting_script_unit_progress_view)
    }
}

fn chapter_shooting_script_view(
    resolved: ResolvedChapterShootingScript,
) -> ChapterShootingScriptView {
    ChapterShootingScriptView {
        record: chapter_shooting_script_record_summary(resolved.shooting_script.record),
        script: resolved.shooting_script.script,
    }
}

fn chapter_shooting_script_unit_progress_view(
    progress: ChapterShootingScriptUnitProgress,
) -> ChapterShootingScriptUnitProgressView {
    ChapterShootingScriptUnitProgressView {
        workflow_run_id: progress.workflow_run_id,
        script: progress.script,
        completed_unit_count: progress.completed_unit_count,
        total_unit_count: progress.total_unit_count,
        completed_record: progress.resolved.map(|resolved| {
            chapter_shooting_script_record_summary(resolved.shooting_script.record)
        }),
    }
}

fn edited_chapter_shooting_script_view(
    edited: EditedChapterShootingScript,
) -> EditedChapterShootingScriptView {
    EditedChapterShootingScriptView {
        shooting_script: ChapterShootingScriptView {
            record: chapter_shooting_script_record_summary(edited.shooting_script.record),
            script: edited.shooting_script.script,
        },
        invalidated_project_ids: edited.invalidated_project_ids,
    }
}

fn chapter_shooting_script_record_summary(
    record: a4n_db_models::models::chapter::chapter_shooting_script::Model,
) -> a4n_api_contract::shooting_script::ChapterShootingScriptRecordSummary {
    a4n_api_contract::shooting_script::ChapterShootingScriptRecordSummary {
        id: record.id,
        revision: record.revision,
        visual_plan_id: record.visual_plan_id,
        visual_plan_version: record.visual_plan_version,
        visual_plan_sha256: record.visual_plan_sha256,
    }
}
