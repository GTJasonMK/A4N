mod assembly;
mod context;
mod contracts;
pub(crate) mod errors;
mod generation;
mod preparation;
mod progress;
mod stage;
mod task;
mod workflow_rules;

pub(crate) use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, GenerateChapterShootingScriptInput,
};
pub(crate) use contracts::{ChapterShootingScriptUnitProgress, ResolvedChapterShootingScript};
pub(in crate::shooting_script) use contracts::{
    ShootingScriptProgressReporter, ShootingScriptStreamReporter,
};
#[cfg(test)]
pub(crate) use contracts::{seed_visual_plan_scene, visual_plan_fixture};
pub(in crate::shooting_script) use workflow_rules::chapter_shooting_script_workflow_run_input;

use a4n_api_contract::shooting_script::{
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
};
use a4n_error::AppError;
use a4n_novel_host_contract::{
    ShootingScriptFullGenerationApplicationPort, ShootingScriptNextUnitGenerationApplicationPort,
};
use tokio_util::sync::CancellationToken;

pub(crate) fn validate_shooting_generation_mode(
    mode: a4n_api_contract::shooting_script::ShootingScriptGenerationMode,
    shot_ids: &[String],
) -> Result<(), AppError> {
    if mode == a4n_api_contract::shooting_script::ShootingScriptGenerationMode::NextScene
        && !shot_ids.is_empty()
    {
        return Err(AppError::InvalidArgument(
            "局部镜头更新请使用 generate_chapter_shooting_script；逐场景续写不接受 shot_ids".into(),
        ));
    }
    Ok(())
}

#[async_trait::async_trait]
impl ShootingScriptFullGenerationApplicationPort for crate::OrchestratorContext {
    async fn start(
        &self,
        user_id: i64,
        input: &a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
    ) -> Result<
        a4n_api_contract::generation_tasks::GenerationTaskStart,
        a4n_api_contract::generation_tasks::GenerationTaskStartError,
    > {
        self.start_shooting_script_generation_task(user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run(
        &self,
        user_id: i64,
        input: a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.run_shooting_script_generation_task(user_id, input, handle)
            .await
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
        let resolved = self
            .generate_chapter_shooting_script_for_user_validated(
                user_id,
                input,
                progress,
                stream_reporter,
                cancel_token,
                active_task_id,
            )
            .await?;
        let plan_sync = self
            .shooting_script_plan_sync(&resolved.shooting_script)
            .await?;
        Ok(ChapterShootingScriptView {
            record: chapter_shooting_script_record_summary(resolved.shooting_script.record),
            script: resolved.shooting_script.script,
            plan_sync,
        })
    }
}

#[async_trait::async_trait]
impl ShootingScriptNextUnitGenerationApplicationPort for crate::OrchestratorContext {
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
